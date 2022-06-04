use std::{rc::Rc, cell::RefCell, ops::Deref};

use libpulse_binding::{mainloop::threaded::Mainloop, context::{Context, State, FlagSet as ContextFlagSet}, callbacks::ListResult};

pub struct PulseHandle {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>
}

pub struct BasicSinkInfo {
    pub name: String,
    pub index: u32
}

pub struct AudioApplication {
    pub name: String,
    pub pid: usize
}

#[derive(Debug)]
pub enum PulseInitializationError {
    NoAlloc,
    LoopStartErr(i32),
    NoServerConnection,
    ContextConnectErr(i32),
    ContextStateErr
}

impl Drop for PulseHandle {
    fn drop(&mut self) {
        self.mainloop.borrow_mut().lock();
        self.context.borrow_mut().disconnect();
        self.mainloop.borrow_mut().unlock();
        self.mainloop.borrow_mut().stop();
    }
}

impl PulseHandle {
    /// Creates a new Pulse handle
    pub fn new() -> Result<PulseHandle, PulseInitializationError> {
        let mainloop = Rc::new(RefCell::new(match Mainloop::new() {
            Some(l) => l,
            None => return Err(PulseInitializationError::NoAlloc)
        }));

        match mainloop.borrow_mut().start() {
            Ok(_) => {},
            Err(e) => return Err(PulseInitializationError::LoopStartErr(e.0))
        }

        // Lock mainloop to create context
        mainloop.borrow_mut().lock();

        let context = Rc::new(RefCell::new(match Context::new(mainloop.borrow_mut().deref(), "tuxphones")  {
            Some(c) => c,
            None => {
                mainloop.borrow_mut().unlock();
                mainloop.borrow_mut().stop();
                return Err(PulseInitializationError::NoAlloc)
            }
        }));

        // State callback to wait for connection
        {
            let ml_ref = Rc::clone(&mainloop);
            let context_ref = Rc::clone(&context);
            context.borrow_mut().set_state_callback(Some(Box::new(move || {
                // Needs to be unsafe to be able to borrow mutably multiple times
                match unsafe { (*context_ref.as_ptr()).get_state() } {
                    State::Ready | State::Failed | State::Terminated => unsafe { (*ml_ref.as_ptr()).signal(false); },
                    _ => {}
                }
            })));
        }

        match context.borrow_mut().connect(None, ContextFlagSet::NOFLAGS, None) {
            Ok(_) => {},
            Err(e) => {
                mainloop.borrow_mut().unlock();
                mainloop.borrow_mut().stop();
                return Err(PulseInitializationError::ContextConnectErr(e.0));
            }
        }

        loop {
            match context.borrow_mut().get_state() {
                State::Ready => break,
                State::Failed | State::Terminated => {
                    mainloop.borrow_mut().unlock();
                    mainloop.borrow_mut().stop();
                    return Err(PulseInitializationError::ContextStateErr)
                },
                _ => mainloop.borrow_mut().wait()
            }
        }
        context.borrow_mut().set_state_callback(None);

        mainloop.borrow_mut().unlock();

        Ok(PulseHandle { context: Rc::clone(&context), mainloop: Rc::clone(&mainloop) })
    }

    /// Gets the sinks connected to the Pulse server
    pub fn get_sinks(self: &mut Self) -> Vec<BasicSinkInfo> {
        self.mainloop.borrow_mut().lock();
        let results: Rc<RefCell<Option<Vec<BasicSinkInfo>>>> = Rc::new(RefCell::new(Some(vec![])));

        let ml_ref = Rc::clone(&self.mainloop);
        let results_ref = Rc::clone(&results);
        let op = self.context.borrow_mut().introspect().get_sink_info_list(move |res| {
            match res {
                ListResult::Item(info) => results_ref.borrow_mut().as_mut().unwrap().push(BasicSinkInfo { 
                    name: info.name.as_ref().map_or(String::from("unknown name"), |n| { n.to_string() }), 
                    index: info.index 
                }),
                ListResult::End | ListResult::Error => unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                },
            }
        });

        // Wait for operation to complete
        loop {
            match op.get_state() {
                libpulse_binding::operation::State::Running => self.mainloop.borrow_mut().wait(),
                libpulse_binding::operation::State::Done | libpulse_binding::operation::State::Cancelled => break
            }
        }

        self.mainloop.borrow_mut().unlock();

        let res = results.borrow_mut().take().unwrap();
        res
    } 

    /// Gets all applications that are producing audio
    pub fn get_audio_applications() -> Vec<AudioApplication> {
        vec![]
    }
}