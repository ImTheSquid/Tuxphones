use std::{rc::Rc, cell::RefCell, ops::Deref};

use libpulse_binding::{mainloop::threaded::Mainloop, context::{Context, State, FlagSet as ContextFlagSet}, callbacks::ListResult, operation::Operation};
use crate::pid;

pub struct PulseHandle {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    audio_is_setup: bool,
    tuxphones_sink_module_index: Option<u32>,
    combined_sink_index: Option<u32>,
    combined_sink_module_index: Option<u32>,
    current_app_info: Option<CurrentAppInfo>
}

struct CurrentAppInfo {
    sink_input_restore_index: u32,
    index: u32
}

pub struct BasicSinkInfo {
    pub name: String,
    pub index: u32,
    module: Option<u32>
}

#[derive(Debug)]
pub struct AudioApplication {
    pub name: String,
    pub pid: pid,
    pub index: u32,
    pub sink_index: u32
}

#[derive(Debug)]
pub enum PulseInitializationError {
    NoAlloc,
    LoopStartErr(i32),
    ContextConnectErr(i32),
    ContextStateErr
}

impl std::fmt::Display for PulseInitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PulseInitializationError::NoAlloc => "Unable to allocate".to_string(),
            PulseInitializationError::LoopStartErr(code) => format!("Loop start error: {}", code),
            PulseInitializationError::ContextConnectErr(code) => format!("Context connection error: {}", code),
            PulseInitializationError::ContextStateErr => "Context state error".to_string(),
        };
        f.write_str(&str)
    }
}

#[derive(Debug)]
pub enum PulseCaptureSetupError {
    NoPassthrough,
    NoDefaultSink
}

impl std::fmt::Display for PulseCaptureSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PulseCaptureSetupError::NoPassthrough => "No passthrough sink found",
            PulseCaptureSetupError::NoDefaultSink => "No default sink found",
        })
    }
}

#[derive(Debug)]
pub enum PulseCaptureError {
    NotSetup,
    NoAppWithPid
}

impl std::fmt::Display for PulseCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PulseCaptureError::NotSetup => "Capture not setup",
            PulseCaptureError::NoAppWithPid => "No app with given PID found",
        })
    }
}

impl Drop for PulseHandle {
    fn drop(&mut self) {
        if self.audio_is_setup {
            self.teardown_audio_capture();
        }

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

        Ok(PulseHandle { 
            context: Rc::clone(&context), 
            mainloop: Rc::clone(&mainloop), 
            audio_is_setup: false, 
            tuxphones_sink_module_index: None, 
            combined_sink_index: None,
            combined_sink_module_index: None,
            current_app_info: None
        })
    }

    /// Gets the sinks connected to the Pulse server
    pub fn get_sinks(self: &mut Self) -> Vec<BasicSinkInfo> {
        self.mainloop.borrow_mut().lock();
        let results = Rc::new(RefCell::new(Some(vec![])));

        let ml_ref = Rc::clone(&self.mainloop);
        let results_ref = Rc::clone(&results);
        let op = self.context.borrow_mut().introspect().get_sink_info_list(move |res| {
            match res {
                ListResult::Item(info) => results_ref.borrow_mut().as_mut().unwrap().push(BasicSinkInfo { 
                    name: info.name.as_ref().map_or(String::from("unknown name"), |n| { n.to_string() }), 
                    index: info.index,
                    module: info.owner_module
                }),
                ListResult::End | ListResult::Error => unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                },
            }
        });

        op_wait(&mut self.mainloop.borrow_mut(), &op);

        self.mainloop.borrow_mut().unlock();

        let res = results.borrow_mut().take().unwrap();
        res
    } 

    /// Gets all applications that are producing audio
    pub fn get_audio_applications(self: &mut Self) -> Vec<AudioApplication> {
        self.mainloop.borrow_mut().lock();

        let results = Rc::new(RefCell::new(Some(vec![])));

        let ml_ref = Rc::clone(&self.mainloop);
        let results_ref = Rc::clone(&results);
        let op = self.context.borrow_mut().introspect().get_sink_input_info_list(move |res| {
            match res {
                ListResult::Item(info) => {
                    if let Some(pid) = info.proplist.get_str("application.process.id") {
                        results_ref.borrow_mut().as_mut().unwrap().push(AudioApplication {
                            name: info.proplist.get_str("application.name").unwrap_or("NONAME".to_string()),
                            pid: pid.parse().unwrap(),
                            index: info.index,
                            sink_index: info.sink,
                        });
                    }
                },
                ListResult::End | ListResult::Error => unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                }
            }
        });

        op_wait(&mut self.mainloop.borrow_mut(), &op);

        self.mainloop.borrow_mut().unlock();
        
        let res = results.borrow_mut().take().unwrap();
        res
    }

    /// Adds sinks for audio capture
    pub fn setup_audio_capture(self: &mut Self, passthrough_override: Option<&str>) -> Result<(), PulseCaptureSetupError> {
        // Don't do the same thing twice
        if self.audio_is_setup {
            return Ok(());
        }

        let passthrough_sink = match passthrough_override {
            Some(s) => s.to_string(),
            None => {
                self.mainloop.borrow_mut().lock();
                let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
                let ml_ref = Rc::clone(&self.mainloop);
                let res_ref = Rc::clone(&result);
                let op = self.context.borrow_mut().introspect().get_sink_info_by_name("@DEFAULT_SINK@", move |info| unsafe {
                    match info {
                        ListResult::Item(sink) => res_ref.borrow_mut().replace(sink.name.as_ref().map_or(String::from("unknown name"), |n| { n.to_string() })),
                        ListResult::End | ListResult::Error => None
                    };
                    
                    (*ml_ref.as_ptr()).signal(false);
                });

                op_wait(&mut self.mainloop.borrow_mut(), &op);
                self.mainloop.borrow_mut().unlock();

                let res = match result.borrow_mut().take() {
                    Some(r) => r,
                    None => return Err(PulseCaptureSetupError::NoDefaultSink)
                };
                res
            }
        };

        let mut tux_sink_found = false;
        let mut tux_combined_sink_found = false;
        let mut passthrough_sink_found = false;

        for sink in self.get_sinks() {
            match &sink.name[..] {
                "tuxphones" => tux_sink_found = true,
                "tuxphones-combined" => tux_combined_sink_found = true,
                n if n == passthrough_sink => passthrough_sink_found = true,
                _ => {}
            }
        }

        if !passthrough_sink_found {
            return Err(PulseCaptureSetupError::NoPassthrough);
        }

        self.mainloop.borrow_mut().lock();
        if !tux_sink_found {
            let ml_ref = Rc::clone(&self.mainloop);
            let op = self.context.borrow_mut().introspect().load_module(
                "module-null-sink", 
                "sink_name=tuxphones sink_properties=device.description=tuxphones", 
                move |_| unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                }
            );

            op_wait(&mut self.mainloop.borrow_mut(), &op);
        }

        if !tux_combined_sink_found {
            let ml_ref = Rc::clone(&self.mainloop);
            // adjust_time=0 prevents a crash for some reason
            let op = self.context.borrow_mut().introspect().load_module(
                "module-combine-sink", 
                &format!("sink_name=tuxphones-combined sink_properties=device.description=tuxphones-combined adjust_time=0 slaves=tuxphones,{}", passthrough_sink), 
                move |_| unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                }
            );

            op_wait(&mut self.mainloop.borrow_mut(), &op);
        }

        self.mainloop.borrow_mut().unlock();

        for sink in self.get_sinks() {
            match &sink.name[..] {
                "tuxphones" => self.tuxphones_sink_module_index = Some(sink.module.unwrap()),
                "tuxphones-combined" => {
                    self.combined_sink_module_index = Some(sink.module.unwrap());
                    self.combined_sink_index = Some(sink.index);
                },
                _ => {}
            }
        }

        self.audio_is_setup = true;
        Ok(())
    }

    /// Removes audio capture sinks
    pub fn teardown_audio_capture(self: &mut Self) {
        if !self.audio_is_setup {
            return;
        }

        self.audio_is_setup = false;

        self.mainloop.borrow_mut().lock();

        if let Some(idx) = self.tuxphones_sink_module_index {
            self.unload_module(idx);
        }

        if let Some(idx) = self.combined_sink_module_index {
            self.unload_module(idx);
        }

        self.tuxphones_sink_module_index = None;
        self.combined_sink_index = None;
        self.combined_sink_module_index = None;

        self.mainloop.borrow_mut().unlock();
    }

    /// Unloads modules
    fn unload_module(self: &mut Self, idx: u32) {
        let ml_ref = Rc::clone(&self.mainloop);
        let op = self.context.borrow_mut().introspect().unload_module(idx, move |_| unsafe {
            (*ml_ref.as_ptr()).signal(false);
        });

        op_wait(&mut self.mainloop.borrow_mut(), &op);
    }

    /// Starts capturing audio from the application with the given Pulse PID
    pub fn start_capture(self: &mut Self, pid: pid) -> Result<(), PulseCaptureError> {
        if !self.audio_is_setup || self.combined_sink_module_index.is_none() {
            return Err(PulseCaptureError::NotSetup);
        }

        for app in self.get_audio_applications() {
            if app.pid == pid {
                self.mainloop.borrow_mut().lock();

                let ml_ref = Rc::clone(&self.mainloop);
                self.current_app_info = Some(CurrentAppInfo {
                    sink_input_restore_index: app.sink_index,
                    index: app.index
                });
                let op = self.context.borrow_mut().introspect().move_sink_input_by_index(app.index, self.combined_sink_index.unwrap(), Some(Box::new(move |_| unsafe {
                    (*ml_ref.as_ptr()).signal(false);
                })));

                op_wait(&mut self.mainloop.borrow_mut(), &op);

                self.mainloop.borrow_mut().unlock();

                return Ok(());
            }
        }

        Err(PulseCaptureError::NoAppWithPid)
    }

    /// Stop capturing audio from application
    pub fn stop_capture(self: &mut Self) {
        self.mainloop.borrow_mut().lock();

        if let Some(info) = &self.current_app_info {
            let ml_ref = Rc::clone(&self.mainloop);
            let op = self.context.borrow_mut().introspect().move_sink_input_by_index(info.index, info.sink_input_restore_index, Some(Box::new(move |_| unsafe {
                (*ml_ref.as_ptr()).signal(false);
            })));

            op_wait(&mut self.mainloop.borrow_mut(), &op);
        }

        self.current_app_info = None;
        self.mainloop.borrow_mut().unlock();
    }
}

/// Wait for operation to complete
fn op_wait<T: ?Sized>(ml: &mut Mainloop, op: &Operation<T>) {
    loop {
        match op.get_state() {
            libpulse_binding::operation::State::Running => ml.wait(),
            libpulse_binding::operation::State::Done | libpulse_binding::operation::State::Cancelled => break
        }
    }
}