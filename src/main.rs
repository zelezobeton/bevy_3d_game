/*
TODO:
- Make character moving gradually, not with steps (add physics)

- Make custom randomly leveled ground (hills, valleys)
- Stack cubes, create buildings
- Spawn creatures that interact with character, chase him, hurt him, etc.

REFACTOR:
- Default camera position should depend on starting player position

DONE:
- Make camera moving smoother
- Make the camera follow the character
- Delete not used features
*/

use bevy::{core::FixedTimestep, ecs::schedule::SystemSet, prelude::*, render::camera::Camera3d};
use rand::Rng;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
    GameOver,
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .init_resource::<Game>()
        .add_plugins(DefaultPlugins)
        .add_state(GameState::Playing)
        .add_startup_system(setup_cameras)
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(move_player)
                .with_system(focus_camera)
        )
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(teardown))
        .add_system_set(SystemSet::on_exit(GameState::GameOver).with_system(teardown))
        .add_system(bevy::input::system::exit_on_esc_system)
        .run();
}

struct Cell {
    height: f32,
}

#[derive(Default)]
struct Player {
    entity: Option<Entity>,
    i: usize,
    j: usize,
    move_cooldown: Timer,
}

#[derive(Default)]
struct Game {
    board: Vec<Vec<Cell>>,
    player: Player,
    camera_is_pos: Vec3,
    camera_should_pos: Vec3,
    player_pos: Vec3,
}

const BOARD_SIZE_I: usize = 14;
const BOARD_SIZE_J: usize = 21;

const DEFAULT_PLAYER_POS: [f32; 3] = [
    BOARD_SIZE_I as f32 / 2.0,
    0.0,
    BOARD_SIZE_J as f32 / 2.0 - 0.5,
];

const DEFAULT_CAMERA_POS: [f32; 3] = [-7.0, 14.0, 10.0];

fn setup_cameras(mut commands: Commands, mut game: ResMut<Game>) {
    game.camera_should_pos = Vec3::from(DEFAULT_CAMERA_POS);
    game.camera_is_pos = game.camera_should_pos; 

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(game.camera_is_pos)
        .looking_at(Vec3::from(DEFAULT_PLAYER_POS), Vec3::Y), // focus rotation of camera on player
        ..default()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut game: ResMut<Game>) {
    // reset the game state
    game.player.i = BOARD_SIZE_I / 2;
    game.player.j = BOARD_SIZE_J / 2;
    game.player.move_cooldown = Timer::from_seconds(0.3, false);

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 10.0, 4.0),
        point_light: PointLight {
            intensity: 3000.0,
            shadows_enabled: true,
            range: 30.0,
            ..default()
        },
        ..default()
    });

    // spawn the game board
    // let cell_scene = asset_server.load("models/AlienCake/tile.glb#Scene0");
    let cell_scene = asset_server.load("models/AlienCake/cliff_block_rock.glb#Scene0");
    game.board = (0..BOARD_SIZE_J)
        .map(|j| {
            (0..BOARD_SIZE_I)
                .map(|i| {
                    let height = rand::thread_rng().gen_range(-0.3..0.3);
                    // let height = 0.0;
                    commands
                        .spawn_bundle(TransformBundle::from(Transform::from_xyz(
                            i as f32,
                            height - 1.0,
                            j as f32,
                        )))
                        .with_children(|cell| {
                            cell.spawn_scene(cell_scene.clone());
                        });
                    Cell { height }
                })
                .collect()
        })
        .collect();

    // spawn the game character
    game.player_pos = Vec3::new(
        game.player.i as f32,
        game.board[game.player.j][game.player.i].height,
        game.player.j as f32,
    );
    game.player.entity = Some(
        commands
            .spawn_bundle(TransformBundle::from(Transform {
                translation: game.player_pos,
                rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                ..default()
            }))
            .with_children(|cell| {
                // cell.spawn_scene(asset_server.load("models/AlienCake/alien.glb#Scene0"));
                cell.spawn_scene(asset_server.load("models/AlienCake/characterDigger.glb#Scene0"));
            })
            .id(),
    );
}

// remove all entities that are not a camera
fn teardown(mut commands: Commands, entities: Query<Entity, Without<Camera>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// control the game character
fn move_player(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut game: ResMut<Game>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
) {
    if game.player.move_cooldown.tick(time.delta()).finished() {
        let mut moved = false;
        let mut rotation = 0.0;

        if keyboard_input.pressed(KeyCode::Up) {
            if game.player.i < BOARD_SIZE_I - 1 {
                game.player.i += 1;
            }
            rotation = -std::f32::consts::FRAC_PI_2;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            if game.player.i > 0 {
                game.player.i -= 1;
            }
            rotation = std::f32::consts::FRAC_PI_2;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            if game.player.j < BOARD_SIZE_J - 1 {
                game.player.j += 1;
            }
            rotation = std::f32::consts::PI;
            moved = true;
        }
        if keyboard_input.pressed(KeyCode::Left) {
            if game.player.j > 0 {
                game.player.j -= 1;
            }
            rotation = 0.0;
            moved = true;
        }

        // move on the board
        if moved {
            game.player.move_cooldown.reset();
            *transforms.get_mut(game.player.entity.unwrap()).unwrap() = Transform {
                translation: Vec3::new(
                    game.player.i as f32,
                    game.board[game.player.j][game.player.i].height,
                    game.player.j as f32,
                ),
                rotation: Quat::from_rotation_y(rotation),
                ..default()
            };
        }
    }
}

// change the focus of the camera
fn focus_camera(
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut transforms: ParamSet<(Query<&mut Transform, With<Camera3d>>, Query<&Transform>)>,
) {
    // Target player with camera if player exists
    if let Some(player_entity) = game.player.entity {
        if let Ok(player_transform) = transforms.p1().get(player_entity) {
            let player_new_pos = player_transform.translation;
            let player_pos_diff = game.player_pos - player_new_pos;
            game.player_pos = player_new_pos;
            game.camera_should_pos = game.camera_should_pos - player_pos_diff;
        }
    // otherwise, target the middle of the board
    } else {
        game.camera_should_pos = Vec3::from(DEFAULT_CAMERA_POS);
    }

    // Smooth movement #1
    // game.camera_is_pos = game.camera_is_pos.lerp(game.camera_should_pos, 0.2);

    // Smooth movement #2
    const SPEED: f32 = 2.0;
    let mut camera_motion = game.camera_should_pos - game.camera_is_pos;
    if camera_motion.length() > 0.2 {
        camera_motion *= SPEED * time.delta_seconds();
        game.camera_is_pos += camera_motion;
    }

    // look at that new camera's actual focus
    for mut transform in transforms.p0().iter_mut() {
        *transform = transform.with_translation(game.camera_is_pos);
    }
}