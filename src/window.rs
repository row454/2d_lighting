use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent as WinitWindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window,
};

pub struct Window {
    event_loop: EventLoop<()>,
    window: window::Window,
}

impl Window {
    pub fn window(&self) -> &window::Window {
        &self.window
    }
    pub fn new() -> Self {
        // TODO: Add size
        let event_loop = EventLoop::new().unwrap();
        let window = window::WindowBuilder::new()
            .with_title("row666 lighting test")
            .build(&event_loop)
            .unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        Self { event_loop, window }
    }
    pub fn run(self, mut callback: impl FnMut(WindowEvent)) {
        self.event_loop
            .run(move |event, control_flow| match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() => match event {
                    WinitWindowEvent::CloseRequested
                    | winit::event::WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => control_flow.exit(),
                    WinitWindowEvent::Resized(physical_size) => callback(WindowEvent::Resized {
                        width: physical_size.width,
                        height: physical_size.height,
                    }),
                    WinitWindowEvent::KeyboardInput { event, .. } => {
                        callback(WindowEvent::Keyboard {
                            state: event.state,
                            keycode: event.physical_key,
                        })
                    }
                    WinitWindowEvent::Focused(false) => callback(WindowEvent::LostFocus),
                    _ => {}
                },
                Event::AboutToWait => callback(WindowEvent::Draw),
                Event::LoopExiting => callback(WindowEvent::Closed),
                _ => {}
            })
            .unwrap();
    }
}

pub enum WindowEvent {
    Resized {
        width: u32,
        height: u32,
    },
    Keyboard {
        state: ElementState,
        keycode: PhysicalKey,
    },
    Draw,
    LostFocus,
    Closed,
}
