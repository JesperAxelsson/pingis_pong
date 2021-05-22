use std::u128;

use bevy::{
    prelude::*,
    render::camera::{ScalingMode, WindowOrigin},
};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use bevy_rapier2d::rapier::na::Vector2;
use bevy_rapier2d::{
    na::Isometry2,
    physics::{EventQueue, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent},
};
use bevy_rapier2d::{
    physics::ColliderHandleComponent,
    rapier::dynamics::{RigidBodyBuilder, RigidBodySet},
};
use rapier2d::geometry::ContactEvent;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Pingis Pong!".to_string(),
            width: ARENA_WIDTH,
            height: ARENA_HEIGHT,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_game.system().label("setup"))
        .add_startup_system(spawn_walls.system().after("setup"))
        .add_startup_system(spawn_paddles.system().after("setup").label("paddles"))
        .add_startup_system(spawn_ball.system().after("setup").label("ball"))
        .add_startup_system(
            load_ui_font
                .system()
                .after("setup")
                .after("paddles")
                .after("ball"),
        )
        .add_system(paddle_movement.system())
        .add_system(print_events.system())
        .add_system(ball_goal.system().label("ball_goal"))
        .add_system(render_scoreboard.system().after("ball_goal"))
        .add_plugin(RapierPhysicsPlugin)
        .run();
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Player {
    Left,
    Right,
}

struct Paddle(f32);
struct Ball(f32);
struct Wall;

const WALL_TOP: u128 = 1;
const WALL_BOTTOM: u128 = 2;

// struct BallTexture(pub Handle<ColorMaterial>) ;
#[derive(Debug, Default)]
pub struct Score {
    pub left: u32,
    pub right: u32,
}

const ARENA_WIDTH: f32 = 1000.;
const ARENA_HEIGHT: f32 = 600.;
const ARENA_MIDDLE: f32 = ARENA_WIDTH / 2.;

const PADDLE_HEIGHT: f32 = 110.0;
const PADDLE_WIDTH: f32 = 15.0;

fn setup_game(
    mut commands: Commands,
    // mut materials: ResMut<Assets<ColorMaterial>>,
    mut rapier_config: ResMut<RapierConfiguration>,
    // asset_server: Res<AssetServer>,
) {
    // Set gravity to 0.0
    rapier_config.gravity = Vector2::zeros();

    // Setup camera
    let mut cam = OrthographicCameraBundle::new_2d();

    cam.orthographic_projection.scaling_mode = ScalingMode::None;
    cam.orthographic_projection.left = 0.;
    cam.orthographic_projection.right = ARENA_WIDTH;
    cam.orthographic_projection.top = ARENA_HEIGHT;
    cam.orthographic_projection.bottom = 0.;
    cam.orthographic_projection.window_origin = WindowOrigin::BottomLeft;

    commands.spawn().insert_bundle(cam);
    commands.spawn_bundle(UiCameraBundle::default());

    // Set physics scale
    rapier_config.scale = 20.0;

    // Load materials
    // let texture_handle = asset_server.load("assets/sprites/ball.png");
    // let material_handle = materials.add(asset_server.load("sprites/ball.png").into());
    // commands.insert_resource(BallTexture(material_handle));
    commands.insert_resource(Score::default());
}

struct UiFont(Handle<Font>);

fn load_ui_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<Font> = asset_server.load("fonts/Pattaya-Regular.ttf");

    // we can store the handle in a resource:
    //  - to prevent the asset from being unloaded
    //  - if we want to use it to access the asset later
    commands.insert_resource(UiFont(handle.clone()));

    // scoreboard
    // Left

    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: handle.clone(),
                        font_size: 96.0,
                        color: Color::rgb(1.0, 1.0, 1.0),
                    },
                }],
                // alignment: TextAlignment {
                //     horizontal: HorizontalAlign::Center,
                //     vertical: VerticalAlign::Center,
                // },
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(ARENA_HEIGHT / 2. - 48.),
                    left: Val::Px(ARENA_MIDDLE - ARENA_WIDTH / 4.),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player::Left);

    // Right
    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: handle.clone(),
                        font_size: 96.0,
                        color: Color::rgb(1.0, 1.0, 1.0),
                    },
                }],
                alignment: TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    vertical: VerticalAlign::Center,
                },
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(ARENA_HEIGHT / 2. - 48.),
                    left: Val::Px(ARENA_MIDDLE + ARENA_WIDTH / 4.),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player::Right);
}

fn spawn_paddles(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    rapier_config: Res<RapierConfiguration>,
    // asset_server: Res<AssetServer>,
) {
    let sprite_size_x = PADDLE_WIDTH;
    let sprite_size_y = PADDLE_HEIGHT;

    let collider_size_x = sprite_size_x / rapier_config.scale;
    let collider_size_y = sprite_size_y / rapier_config.scale;

    let wall_offset = 50.;

    let body = RigidBodyBuilder::new_dynamic()
        .translation(
            wall_offset / rapier_config.scale,
            ARENA_HEIGHT / 2. / rapier_config.scale,
        )
        // .lock_translations()
        .ccd_enabled(true)
        .lock_rotations();

    let density = 20.;
    let restitution = 1.0;
    let friction = -0.5;
    let paddle_speed = 600.0;

    // Spawn entity with `Player` struct as a component for access in movement query.
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(sprite_size_x, sprite_size_y)),
            ..Default::default()
        })
        .insert(body)
        .insert(
            ColliderBuilder::cuboid(collider_size_x / 2.0, collider_size_y / 2.0)
                .density(density)
                .friction(friction)
                .restitution(restitution),
        )
        .insert(Paddle(paddle_speed))
        .insert(Player::Left);

    // *** LEFT ***
    let body = RigidBodyBuilder::new_dynamic()
        .translation(
            (ARENA_WIDTH - wall_offset) / rapier_config.scale,
            ARENA_HEIGHT / 2. / rapier_config.scale,
        )
        // .lock_translations()
        .ccd_enabled(true)
        .lock_rotations();

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(sprite_size_x, sprite_size_y)),
            ..Default::default()
        })
        .insert(body)
        .insert(
            ColliderBuilder::cuboid(collider_size_x / 2.0, collider_size_y / 2.0)
                .density(density)
                .friction(friction)
                .restitution(restitution),
        )
        .insert(Paddle(paddle_speed))
        .insert(Player::Right);
}

fn spawn_ball(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    rapier_config: Res<RapierConfiguration>,
    asset_server: Res<AssetServer>,
    // ball_texture:  Res<BallTexture>,
) {
    let material_handle = materials.add(asset_server.load("sprites/ball.png").into());

    let sprite_size_x = 40.0;
    let sprite_size_y = 40.0;

    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
    let collider_size_x = sprite_size_x / rapier_config.scale;

    let body = RigidBodyBuilder::new_dynamic()
        .translation(
            ARENA_WIDTH / 2. / rapier_config.scale,
            ARENA_HEIGHT / 2. / rapier_config.scale,
        )
        .linvel(10., 10.)
        .angular_damping(-0.01)
        // .linear_damping(-0.2)
        .can_sleep(false)
        .ccd_enabled(true);

    let density = 0.001;
    // let density = 5.0;
    let restitution = 1.1;
    let friction = 1.4;

    // Spawn entity with `Player` struct as a component for access in movement query.
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: material_handle.clone(),
            // material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(sprite_size_x, sprite_size_y)),
            ..Default::default()
        })
        .insert(body)
        .insert(
            ColliderBuilder::ball(collider_size_x / 2.0)
                .friction(friction)
                .restitution(restitution)
                .density(density),
        )
        .insert(Ball(10.0));
}

fn spawn_walls(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    rapier_config: Res<RapierConfiguration>,
) {
    let sprite_size_x = ARENA_WIDTH;
    let sprite_size_y = 20.0;

    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
    let collider_size_x = sprite_size_x / rapier_config.scale;
    let collider_size_y = sprite_size_y / rapier_config.scale;

    let density = 1.0;
    let restitution = 1.0;
    let friction = -1.0;

    // Bottom
    let b = RigidBodyBuilder::new_static()
        .translation(
            sprite_size_x / 2. / rapier_config.scale,
            sprite_size_y / 2. / rapier_config.scale,
        )
        .lock_rotations();

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(sprite_size_x, sprite_size_y)),
            // transform: trans,
            ..Default::default()
        })
        .insert(b)
        .insert(
            ColliderBuilder::cuboid(collider_size_x / 2.0, collider_size_y / 2.0)
                .density(density)
                .friction(friction)
                .restitution(restitution)
                .user_data(WALL_BOTTOM)
        )
        .insert(Wall);

    // Top
    let b = RigidBodyBuilder::new_static()
        .translation(
            sprite_size_x / 2. / rapier_config.scale,
            (ARENA_HEIGHT - sprite_size_y / 2.) / rapier_config.scale,
        )
        .lock_rotations();

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(sprite_size_x, sprite_size_y)),
            // transform: trans,
            ..Default::default()
        })
        .insert(b)
        .insert(
            ColliderBuilder::cuboid(collider_size_x / 2.0, collider_size_y / 2.0)
                .density(density)
                .friction(friction)
                .restitution(restitution)
                .user_data(WALL_TOP)
        )
        .insert(Wall);
}

fn paddle_movement(
    keyboard_input: Res<Input<KeyCode>>,
    rapier_parameters: Res<RapierConfiguration>,
    mut rigid_bodies: ResMut<RigidBodySet>,
    player_info: Query<(&Paddle, &Transform, &RigidBodyHandleComponent, &Player)>,
) {
    // let lim_top = 20.;
    // let lim_bottom= ARENA_HEIGHT -20.;

    for (paddle, _transform, rigid_body_component, player) in player_info.iter() {
        let (x_axis, y_axis) = match player {
            Player::Left => {
                let x_axis = -(keyboard_input.pressed(KeyCode::A) as i8)
                    + (keyboard_input.pressed(KeyCode::D) as i8);
                let y_axis = -(keyboard_input.pressed(KeyCode::S) as i8)
                    + (keyboard_input.pressed(KeyCode::W) as i8);
                (x_axis, y_axis)
            }
            Player::Right => {
                let x_axis = -(keyboard_input.pressed(KeyCode::Numpad4) as i8)
                    + (keyboard_input.pressed(KeyCode::Numpad6) as i8);
                let y_axis = -(keyboard_input.pressed(KeyCode::Numpad5) as i8)
                    + (keyboard_input.pressed(KeyCode::Numpad8) as i8);
                (x_axis, y_axis)
            }
        };

        let mut move_delta = Vector2::new(x_axis as f32, y_axis as f32);
        if move_delta != Vector2::zeros() {
            // Note that the RapierConfiguration::Scale factor is also used here to transform
            // the move_delta from: 'pixels/second' to 'physics_units/second'
            move_delta /= move_delta.magnitude() * rapier_parameters.scale;
        }

        // Update the velocity on the rigid_body_component,
        // the bevy_rapier plugin will update the Sprite transform.
        if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
            // Move paddle
            rb.set_linvel(move_delta * paddle.0, true);

            // Clamp paddle
            let pos = rb.position();
            // let delta = move_delta * paddle.0;

            let (lim_left, lim_right) = match player {
                Player::Left => (PADDLE_WIDTH, ARENA_MIDDLE - PADDLE_WIDTH),
                Player::Right => (ARENA_MIDDLE + PADDLE_WIDTH, ARENA_WIDTH - PADDLE_WIDTH),
            };

            // Scale to physics engine
            let (lim_left, lim_right) = (
                lim_left / rapier_parameters.scale,
                lim_right / rapier_parameters.scale,
            );

            if pos.translation.x < lim_left {
                // println!("delta l {:?} ", delta);
                let mut trans = pos.translation.clone();
                trans.x = lim_left;
                rb.set_position(trans.into(), true);
            } else if pos.translation.x > lim_right {
                // println!("delta r {:?} ", delta);
                let mut trans = pos.translation.clone();
                trans.x = lim_right;
                rb.set_position(trans.into(), true);
            }
        }

        // *** Angle the paddle **
        let rotation_direction = match player {
            Player::Left => {
                -(keyboard_input.pressed(KeyCode::E) as i8)
                    + (keyboard_input.pressed(KeyCode::Q) as i8)
            }
            Player::Right => {
                -(keyboard_input.pressed(KeyCode::Numpad9) as i8)
                    + (keyboard_input.pressed(KeyCode::Numpad7) as i8)
            }
        };

        if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
            let rotation = rotation_direction as f32 * 3.;
            let cur_angle = rb.position().rotation.angle();

            if rotation > 0. && cur_angle <= 0.8 {
                rb.set_angvel(rotation, true);
            } else if rotation < 0. && cur_angle >= -0.8 {
                rb.set_angvel(rotation, true);
            } else {
                rb.set_angvel(0.0, true);
            }

            // println!("Angle: {:?} {} {}", player, cur_angle, rotation);
        }
    }
}

fn print_events(
    events: Res<EventQueue>,
    ball_info: Query<(&Ball, &ColliderHandleComponent, &RigidBodyHandleComponent)>,
) {
    while let Ok(_intersection_event) = events.intersection_events.pop() {
        // println!("Received intersection event: {:?}", intersection_event);
    }

    while let Ok(contact_event) = events.contact_events.pop() {
        if let ContactEvent::Stopped(h1, h2) = contact_event {
            for (_ball, col_handle, _rigid_body) in ball_info.iter() {
                if col_handle.handle() == h1 || col_handle.handle() == h2 {
                    // println!("Ball collided with x");
                }
            }
        }
        // let ball = ball_info.iter().try_find(|i| i.)
        // println!("Received contact event: {:?}", contact_event);
    }
}

fn ball_goal(
    rapier_config: Res<RapierConfiguration>,
    mut rigid_bodies: ResMut<RigidBodySet>,
    mut score: ResMut<Score>,
    ball_info: Query<(&Ball, &Transform, &RigidBodyHandleComponent)>,
) {
    let lim_left = 0.;
    let lim_right = ARENA_WIDTH;

    for (_ball, transform, rigid_body_component) in ball_info.iter() {
        let reset_ball = if transform.translation.x < lim_left {
            score.right += 1;
            // println!("GOAL, point right! {:?}", *score);
            true
        } else if transform.translation.x > lim_right {
            score.left += 1;
            // println!("GOAL, point left! {:?}", *score);
            true
        } else {
            false
        };

        if reset_ball {
            if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
                let x = ARENA_WIDTH / 2. / rapier_config.scale;
                let y = ARENA_HEIGHT / 2. / rapier_config.scale;
                let start_pos = Isometry2::translation(x, y);

                // let x = fastrand::i32(2..3);
                let angle = fastrand::f32() * std::f32::consts::PI * 2.;
                let x = f32::cos(angle);
                let y = f32::sin(angle);
                let speed = Vector2::new(x, y) * 20.0;

                rb.set_linvel(speed, true);
                rb.set_position(start_pos, true);
                // println!("Ball reset");
            }
        }
    }
}

fn render_scoreboard(score: Res<Score>, mut query: Query<(&mut Text, &Player)>) {
    // let mut text = query.single_mut().unwrap();
    for (mut text, player) in query.iter_mut() {
        match player {
            Player::Left => text.sections[0].value = format!("{}", score.left),
            Player::Right => text.sections[0].value = format!("{}", score.right),
        }
    }
}
