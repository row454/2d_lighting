use std::{ops::{Add, AddAssign}, time::Instant};
use assets::TextureAtlasStorage;
use hecs::World;
use input::{Control, InputHandler};
use renderer::RendererState;
use texture_atlas::{DeferredTextureRegion, TextureRegion};
use winit::{dpi::PhysicalSize, event::{ElementState, Event, KeyEvent, WindowEvent}, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowBuilder}};

mod texture;
mod camera;
mod renderer;
mod texture_atlas;
mod assets;
mod input;

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().with_min_inner_size(PhysicalSize::new(640, 360)).build(&event_loop).unwrap();
    let mut game = Game::new(&window).await;
    let mut fps = 0;
    let mut delta_sum = 0;
    let mut previous_time = Instant::now();

    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == game.renderer.window().id() => if !game.input(event) { match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => control_flow.exit(),
            WindowEvent::Resized(physical_size) => {
                game.renderer.resize(*physical_size);
                game.input_handler.reset_states();
            },
            WindowEvent::KeyboardInput { event, .. } => game.input_handler.handle_input(event),
            WindowEvent::Focused(false) => game.input_handler.reset_states(),
            WindowEvent::Moved(_) => game.input_handler.reset_states(),
            _ => {}
        }}
        Event::AboutToWait => {
            delta_sum += previous_time.elapsed().as_nanos();
            previous_time = Instant::now();
            if delta_sum > 1_000_000_000 {
                println!("{}", fps);
                fps = 0;
                delta_sum -= 1_000_000_000;
            }

            game.update();
            match game.renderer.render() {
                Ok(()) => {},
                Err(wgpu::SurfaceError::Lost) => game.renderer.resize(game.renderer.size),
                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                Err(e) => eprintln!("{e:?}")
            }
            fps += 1
        }
        _ => {}
    });
}

#[allow(dead_code)]
struct Game<'a> {
    renderer: RendererState<'a>,
    textures: TextureAtlasStorage,
    world: World,
    input_handler: InputHandler,
}
struct Position(Vec2);
struct Velocity(Vec2);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Vec2 {
    x: f32,
    y: f32,
}
impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}
impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Add<(f32, f32)> for Vec2 {
    type Output = Self;
    fn add(self, rhs: (f32, f32)) -> Self::Output {
        Self {
            x: self.x + rhs.0,
            y: self.y + rhs.1,
        }
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl AddAssign<(f32, f32)> for Vec2 {
    fn add_assign(&mut self, rhs: (f32, f32)) {
        self.x += rhs.0;
        self.y += rhs.1;
    }
}
struct PlayerControlled;

impl<'a> Game<'a> {
    async fn new(window: &'a Window) -> Game<'a> {
        let renderer = RendererState::new(window).await;
        let mut textures = TextureAtlasStorage::new();
        let entities = textures.load("entities", &renderer.texture_creator()).unwrap();
        let mut world = World::new();
        world.spawn((Position((0.0, 0.0).into()), entities.get_region("player").unwrap().unwrap_pair(), PlayerControlled, Velocity((0., 0.).into())));
        world.spawn((Position((10.0, 0.0).into()), entities.get_region("zombie").unwrap().unwrap_pair()));

        let mut input_handler = InputHandler::new();
        input_handler.register_control(KeyCode::KeyW, Control::MoveUp);
        input_handler.register_control(KeyCode::KeyA, Control::MoveLeft);
        input_handler.register_control(KeyCode::KeyS, Control::MoveDown);
        input_handler.register_control(KeyCode::KeyD, Control::MoveRight);



        Game {
            renderer,
            textures,
            world,
            input_handler
        }
        
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { device_id: _, position } => {
                self.renderer.clear_color = wgpu::Color {
                    r: position.x/self.renderer.size.width as f64 - position.x/self.renderer.size.width as f64 * position.y/self.renderer.size.height as f64,
                    g: position.x/self.renderer.size.width as f64 * position.y/self.renderer.size.height as f64,
                    b: position.y/self.renderer.size.height as f64 - position.x/self.renderer.size.width as f64 * position.y/self.renderer.size.height as f64,
                    a: 1.0,
                };
            },
            _ => return false
        }
        true
    }
    fn update(&mut self) {

        for (_, (vel, _)) in self.world.query_mut::<(&mut Velocity, &PlayerControlled)>() {
            vel.0 = (0., 0.).into();
            if self.input_handler.is_pressed(Control::MoveUp) {
                vel.0 += (0., 1.)
            }
            if self.input_handler.is_pressed(Control::MoveDown) {
                vel.0 += (0., -1.)
            }
            if self.input_handler.is_pressed(Control::MoveLeft) {
                vel.0 += (-1., 0.)
            }
            if self.input_handler.is_pressed(Control::MoveRight) {
                vel.0 += (1., 0.)
            }
        }
        for (_, (pos, vel)) in self.world.query_mut::<(&mut Position, &Velocity)>() {
            pos.0 += vel.0
        }

        for (_, (pos, sprite)) in self.world.query::<(&Position, &DeferredTextureRegion)>().iter() {
            self.renderer.draw_deferred_sprite((pos.0.x, pos.0.y, 0.), sprite.clone())
        }
        //self.renderer.draw_sprite((0.0, 0.0, 0.0), self.textures.load("entities", &self.renderer.texture_creator()).unwrap().get_region("target").unwrap().unwrap_single());
        //self.renderer.draw_sprite((20.0, 0.0, 0.0), self.textures.load("entities", &self.renderer.texture_creator()).unwrap().get_region("snowball").unwrap().unwrap_single());
        
        self.input_handler.update();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DeferredVertex {
    position: [f32; 3],
    albedo_coords: [f32; 2],
    normal_coords: [f32; 2],
}

impl DeferredVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
 
 
