/*
TODO:
- Add items that player can collect

- Spawn creatures that interact with character, chase him, hurt him, etc.
- Add levels with different layout, platforms etc. 
- Add movement around Y-axis with mouse
- Refine player movement

DONE:
- Rotate enemy to direction it's moving
- Make enemy move towards player
- Rotate character to direction it's moving
- Add human mesh to character
- Merge code back to original git repo
- Make camera focus and follow character
- Move character with keyboard using velocity
- Add jump
*/

use bevy::prelude::*;
use bevy::render::mesh::shape as render_shape;
use bevy_rapier3d::prelude::*;
use std::f32::consts::{PI, FRAC_PI_2};

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct Enemy;

// Should I work with entity IDs for these entities?
// #[derive(Default)]
// struct Game {
//     player: Option<Entity>,
//     camera: Option<Entity>
// }

const DEFAULT_PLAYER_POS: [f32; 3] = [ 0.0, 1.0, 0.0];
const DEFAULT_CAMERA_POS: [f32; 3] = [-7.0, 10.0, 0.0];

fn move_camera(
    mut camera_transform: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<Camera>)>
) {
    let player_pos = player_transform.single_mut().translation;

    let camera_distance: Vec3 = Vec3::from(DEFAULT_CAMERA_POS) - Vec3::from(DEFAULT_PLAYER_POS);
    let new_camera_pos = player_pos + camera_distance;

    // Interpolated camera movement
    camera_transform.single_mut().translation = camera_transform.single_mut().translation.lerp(new_camera_pos, 0.2);
}

fn move_player(
    keyboard_input: Res<Input<KeyCode>>,
    mut velocities: Query<&mut Velocity, With<Player>>,
    mut player_transform: Query<&mut Transform, With<Player>>
) {
    const SPEED: f32 = 7.0;
    let mut vel = velocities.single_mut();
    if keyboard_input.pressed(KeyCode::Up) {
        vel.linvel[0] = SPEED;
        player_transform.single_mut().rotation = Quat::from_rotation_y(0.0);
    }
    else if keyboard_input.pressed(KeyCode::Down) {
        vel.linvel[0] = -SPEED;
        player_transform.single_mut().rotation = Quat::from_rotation_y(PI);
    }
    else if keyboard_input.pressed(KeyCode::Left) {
        vel.linvel[2] = -SPEED;
        player_transform.single_mut().rotation = Quat::from_rotation_y(FRAC_PI_2);
    }
    else if keyboard_input.pressed(KeyCode::Right) {
        vel.linvel[2] = SPEED;
        player_transform.single_mut().rotation = Quat::from_rotation_y(-FRAC_PI_2);
    }
    else {
        vel.linvel[0] = 0.0;
        vel.linvel[2] = 0.0;
    }
    
    if keyboard_input.just_pressed(KeyCode::Space) {
        vel.linvel[1] = 5.0;
    }

    // Custom gravity
    // vel.linvel[1] -= 1.0;
}

fn move_enemy(
    mut velocities: Query<&mut Velocity, With<Enemy>>,
    mut enemy_transform: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<Enemy>)>
) {
    const SPEED: f32 = 6.0;
    let mut vel = velocities.single_mut();

    // Get vector representing direction from enemy to player
    let mut direction_vec = player_transform.single_mut().translation - enemy_transform.single_mut().translation;
    direction_vec = direction_vec.normalize();

    // Calculate distance of vectors, so enemy chases player only until it's near him
    let vec2_player = Vec2::new(player_transform.single_mut().translation[0], player_transform.single_mut().translation[2]);
    let vec2_enemy = Vec2::new(enemy_transform.single_mut().translation[0], enemy_transform.single_mut().translation[2]);
    if vec2_player.distance(vec2_enemy) > 1.4 {
        vel.linvel[0] = direction_vec[0] * SPEED;
        vel.linvel[2] = direction_vec[2] * SPEED;
    }

    // Rotate ememy in direction it's moving
    let dir_angle = (direction_vec.x).atan2(direction_vec.z);
    enemy_transform.single_mut().rotation = Quat::from_rotation_y(dir_angle);
}

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::from(DEFAULT_CAMERA_POS))
                .looking_at(Vec3::from(DEFAULT_PLAYER_POS), Vec3::Y), // focus rotation of camera on player
            ..default()
        })
        .insert(Camera);
}

fn setup_light(mut commands: Commands) {
    const HALF_SIZE: f32 = 100.0;

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            // Configure the projection to better fit the scene
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 100.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(10.0, 2.0, 10.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });
}

fn spawn_level(
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    commands: &mut Commands
) {
    let ground_size = 20.1;
    let ground_height = 0.1;

    // Spawn ground
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(render_shape::Box::new(
                ground_size,
                ground_height,
                ground_size,
            ))),
            material: materials.add(Color::DARK_GREEN.into()),
            transform: Transform::from_xyz(0.0, -ground_height, 0.0),
            ..default()
        })
        .insert(Collider::cuboid(ground_size/2.0, ground_height/2.0, ground_size/2.0))
        .insert(Transform::from_xyz(0.0, -ground_height, 0.0))
        .insert(GlobalTransform::default());
    
    // Spawn platform
    commands
        .spawn(PbrBundle {
            mesh		: meshes.add(Mesh::from(render_shape::Box::new(5.0, 0.5, 5.0))),
            material	: materials.add(Color::YELLOW.into()),
            transform	: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        })
        .insert(RigidBody::Fixed)
        .insert(Collider::cuboid(5.0/2.0, 0.5/2.0, 5.0/2.0))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_translation(Vec3::from([5.0, 3.0, 5.0]))));
    
    // Spawn cubes
    let cube = render_shape::Cube::default();
    let cubes_positions = [[2.0, 1.0, 2.0], [-2.0, 1.0, 2.0], [2.0, 1.0, -2.0], [-2.0, 1.0, -2.0]];
    for pos in cubes_positions {
        let vec_pos = Vec3::from(pos);
        commands
            .spawn(PbrBundle {
                mesh		: meshes.add(Mesh::from(cube)),
                material	: materials.add(Color::CRIMSON.into()),
                transform	: Transform::from_xyz(0.0, 1.0, 0.0),
                ..default()
            })
            .insert(RigidBody::Dynamic)
            .insert(Collider::cuboid(0.5, 0.5, 0.5))
            .insert(Restitution::coefficient(0.7))
            .insert(TransformBundle::from(Transform::from_translation(vec_pos)));
    }

    // Spawn sphere
    let sphere = render_shape::Capsule {
        depth: 0.0,
        ..default()
    };
    commands
        .spawn(PbrBundle {
            mesh		: meshes.add(Mesh::from(sphere)),
            material	: materials.add(Color::GOLD.into()),
            transform	: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(0.5))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(1.0, 0.0, 1.0)));
    
    // Spawn alien
    commands
        .spawn(Enemy)
        .insert(PbrBundle {
            transform: Transform::from_xyz(-5.0, 1.0, -5.0),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn(SceneBundle {
                scene: asset_server.load("models/AlienCake/alien.glb#Scene0"), 
                transform: Transform {
                    translation: Vec3::new(0.0, -1.0, 0.0),
                    rotation: Quat::from_rotation_y(PI),
                    scale:  Vec3::new(2.0, 2.0, 2.0),
                },
                ..default()});
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero())
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Collider::capsule_y(0.5, 0.5))
        .insert(Restitution::coefficient(0.7));
}

fn spawn_player(
    asset_server: &Res<AssetServer>,
    commands: &mut Commands,
) {
    commands
        .spawn(Player)
        .insert	(PbrBundle {
			transform: Transform::from_xyz(0.0, 1.0, 0.0),
			..default()
		})
        .with_children(|cell| {
            cell.spawn(SceneBundle {
                scene: asset_server.load("models/AlienCake/characterDigger.glb#Scene0"), 
                transform: Transform {
                    translation: Vec3::new(0.0, -1.0, 0.0),
                    rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                    scale:  Vec3::new(2.0, 2.0, 2.0),
                },
                ..default()});
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero())
        .insert(LockedAxes::ROTATION_LOCKED)
        // .insert(GravityScale(0.0)) // May be needed when tweaking movement for RigidBody::Dynamic
        // .insert(Collider::ball(0.5))
        .insert(Collider::capsule_y(0.5, 0.5))
        .insert(Restitution::coefficient(0.7));
}

fn setup_physics(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands
) {
    spawn_level(&asset_server, &mut meshes, &mut materials, &mut commands);
    spawn_player(&asset_server, &mut commands);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_state(GameState::Playing)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_light)
        .add_startup_system(setup_physics)
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(move_player)
                .with_system(move_camera)
                .with_system(move_enemy)
        )
        .run();
}