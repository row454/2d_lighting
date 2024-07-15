use assets::TextureAtlasStorage;
use gfx::pass::lighting_pass::{Color, DynamicLight};
use hecs::World;
use input::{Control, InputHandler};
use renderer::RendererState;
use row666_metroidbrainia_macros::Vertex;
use std::{
    ops::{Add, AddAssign, Neg},
    time::Instant,
};
use texture_atlas::{DeferredTextureRegion, TextureRegion};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod assets;
mod camera;
mod input;
mod renderer;
mod texture;
mod texture_atlas;
mod window;
mod gfx;


pub async fn run() {
    env_logger::init();
    let window = window::Window::new();
    let mut fps = 0;
    let mut delta_sum = 0;
    let mut previous_time = Instant::now();
    let mut game = Game::new(window.window()).await;
    window.run(move |event| match event {
        window::WindowEvent::Resized { width, height } => game.renderer.resize(width, height),
        window::WindowEvent::Keyboard { state, keycode } => game.input_handler.handle_input(keycode, state),
        window::WindowEvent::Draw => {
            game.update();
            game.renderer.render().unwrap();
        },
        window::WindowEvent::LostFocus => game.input_handler.reset_states(),
        _ => (),
    });
}

#[allow(dead_code)]
struct Game {
    renderer: RendererState,
    texture_storage: TextureAtlasStorage,
    world: World,
    input_handler: InputHandler,
}
struct Position(Vec2);
struct Velocity(Vec2);
struct Acceleration(Vec2);

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
impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
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
struct RandomDrift {
    current_dir: (f32, f32)
}

#[derive(Clone, Copy)]
struct Light {
    radius: f32,
    color: Color
}
impl Game {
    async fn new(window: &Window) -> Game {
        let mut texture_storage = TextureAtlasStorage::new();
        let renderer = RendererState::new(window, &mut texture_storage).await;
        let textures = texture_storage
            .load("textures", &renderer.texture_creator())
            .unwrap();
        let entities = textures.get_region("entities").unwrap().unwrap_atlas();
        let mut world = World::new();
        world.spawn((
            Position((0.0, 0.0).into()),
            entities.get("zombie").unwrap().unwrap_pair(),
            PlayerControlled,
            Velocity((0., 0.).into()),
        ));
        world.spawn((
            Position((0.0, 0.0).into()),
            Velocity((0.1, 0.1).into()),
            Acceleration((0.0, 0.0).into()),
            RandomDrift {
                current_dir: (1.0, 1.0)
            },
            Light {
                radius: 40.0,
                color: Color::from_rgb(20, 50, 130),
            },
        ));
        world.spawn((
            Position((60.0, 40.0).into()),
            Velocity((-0.1, -0.1).into()),
            Acceleration((0.0, 0.0).into()),
            RandomDrift {
                current_dir: (1.0, 1.0)
            },
            Light {
                radius: 40.0,
                color: Color::from_rgb(0, 140, 60),
            },
        ));
        world.spawn((
            Position((-60.0, 90.0).into()),
            Velocity((0.1, -0.1).into()),
            Acceleration((0.0, 0.0).into()),
            RandomDrift {
                current_dir: (1.0, 1.0)
            },
            Light {
                radius: 40.0,
                color: Color::from_rgb(80, 10, 10),
            },
        ));
        world.spawn((
            Position((-60.0, 50.0).into()),
            Velocity((0.1, -0.1).into()),
            Acceleration((0.0, 0.0).into()),
            RandomDrift {
                current_dir: (1.0, 1.0)
            },
            Light {
                radius: 40.0,
                color: Color::from_rgb(100, 70, 70),
            },
        ));


        let mut input_handler = InputHandler::new();
        input_handler.register_control(KeyCode::KeyW, Control::MoveUp);
        input_handler.register_control(KeyCode::KeyA, Control::MoveLeft);
        input_handler.register_control(KeyCode::KeyS, Control::MoveDown);
        input_handler.register_control(KeyCode::KeyD, Control::MoveRight);

        Game {
            renderer,
            texture_storage,
            world,
            input_handler,
        }
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
        for (_, (acc, pos, drift)) in self.world.query_mut::<(&mut Acceleration, &Position, &mut RandomDrift)>() {
            if pos.0.x * drift.current_dir.0 > 0.0 {
                drift.current_dir.0 = -drift.current_dir.0;
                acc.0.x = (rand::random::<f32>() + 1.0) * drift.current_dir.0 * 0.001
            }
            if pos.0.y * drift.current_dir.1 > 0.0 {
                drift.current_dir.1 = -drift.current_dir.1;
                acc.0.y = (rand::random::<f32>() + 1.0) * drift.current_dir.1 * 0.001
            }
        }
        for (_, (vel, acc)) in self.world.query_mut::<(&mut Velocity, &Acceleration)>() {
            vel.0 += acc.0
        }
        for (_, (pos, vel)) in self.world.query_mut::<(&mut Position, &Velocity)>() {
            pos.0 += vel.0
        }

        for (_, (pos, sprite)) in self
            .world
            .query::<(&Position, &DeferredTextureRegion)>()
            .iter()
        {
            self.renderer
                .draw_deferred_sprite((pos.0.x, pos.0.y, 0.), sprite.clone())
        }
        for (_, (pos, &light,)) in self.world.query_mut::<(&Position, &Light,)>() {
            self.renderer.draw_light(DynamicLight {
                center: (pos.0.x, pos.0.y, 10.0),
                radius: light.radius,
                color: light.color,
            });
        }
        //self.renderer.draw_sprite((0.0, 0.0, 0.0), self.textures.load("entities", &self.renderer.texture_creator()).unwrap().get_region("target").unwrap().unwrap_single());
        //self.renderer.draw_sprite((20.0, 0.0, 0.0), self.textures.load("entities", &self.renderer.texture_creator()).unwrap().get_region("snowball").unwrap().unwrap_single());

        self.input_handler.update();
    }
}




trait Vertex {
    type Attribs;
    const ATTRIBS: Self::Attribs;
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
