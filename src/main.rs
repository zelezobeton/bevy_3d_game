/*
TODO:
- Try adding human mesh to character

- Spawn creatures that interact with character, chase him, hurt him, etc.
- Refine player movement
- Add movement around Y-axis with mouse

DONE:
- Merge code back to original git repo
- Make camera focus and follow character
- Move character with keyboard using velocity
- Add jump
*/

use bevy::prelude::*;
use bevy::render::mesh::shape as render_shape;
use bevy_rapier3d::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Camera;

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
    mut velocities: Query<&mut Velocity, With<Player>>
) {
    const SPEED: f32 = 7.0;
    let mut vel = velocities.single_mut();
    if keyboard_input.pressed(KeyCode::Up) {
        vel.linvel[0] = SPEED;
    }
    else if keyboard_input.pressed(KeyCode::Down) {
        vel.linvel[0] = -SPEED;
    }
    else if keyboard_input.pressed(KeyCode::Left) {
        vel.linvel[2] = -SPEED;
    }
    else if keyboard_input.pressed(KeyCode::Right) {
        vel.linvel[2] = SPEED;
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

fn spawn_ground(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    commands: &mut Commands
) {
    let ground_size = 20.1;
    let ground_height = 0.1;

    let ground = commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(render_shape::Box::new(
                ground_size,
                ground_height,
                ground_size,
            ))),
            // material	: materials.add(Color::rgb(0.8, 0.8, 0.8).into()),
            material: materials.add(Color::DARK_GREEN.into()),
            transform: Transform::from_xyz(0.0, -ground_height, 0.0),
            ..Default::default()
        })
        .insert(Collider::cuboid(ground_size/2.0, ground_height/2.0, ground_size/2.0))
        .insert(Transform::from_xyz(0.0, -ground_height, 0.0))
        .insert(GlobalTransform::default())
        .id();
    
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
}

fn spawn_player(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    commands: &mut Commands,
) {
    let capsule = render_shape::Capsule {
        depth: 1.0,
        ..default()
    };
    commands
        .spawn(Player)
        .insert	(PbrBundle {
			mesh		: meshes.add(Mesh::from(capsule)),
			material	: materials.add(Color::NAVY.into()),
			transform	: Transform::from_xyz(0.0, 1.0, 0.0),
			..Default::default()
		})
        // .insert_bundle(TransformBundle {
        //     local: Transform::from_xyz(0.0, 1.0, 0.0),
        //     global: GlobalTransform::identity(),
        // })
        // .with_children(|cell| {
        //     cell.spawn_scene(asset_server.load("models/AlienCake/characterDigger.glb#Scene0"));
        // })
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
    spawn_ground(&mut meshes, &mut materials, &mut commands);
    spawn_player(&mut meshes, &mut materials, asset_server, &mut commands);
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
        )
        .run();
}