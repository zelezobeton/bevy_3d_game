/*
TODO:
- Add different type of enemy, that stands still and shoots bullets

- Make custom character in Blender and animate player attacking 

- Make enemy attack player
  - Make player attack enemy
  - Subtract HP of enemy
- When enemy has 0 HP, despawn it

LONGTERM:
- Spawn creatures that interact with character, chase him, hurt him, etc.
- Add movement around Y-axis with mouse
- Add levels with different layout, platforms etc. 
- Refine player movement

DONE:
- Added intersect_with_shape for getting collision for getting bonus and melee attack 
- Make character shoot bullets
- Rotate player using mouse
- Prevent double-jump
- Add items that player can collect
- Rotate enemy to direction it's moving
- Make enemy move towards player
- Rotate character to direction it's moving
- Add human mesh to character
- Merge code back to original git repo
- Make camera focus and follow character
- Move character with keyboard using velocity
- Add jump
*/

use bevy::{ecs::schedule::SystemSet, prelude::*};
use bevy::render::mesh::shape as render_shape;
use bevy_rapier3d::prelude::*;
use std::f32::consts::{PI, FRAC_PI_2};
use rand::Rng;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Playing,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Cursor;

#[derive(Component)]
struct Bullet(Vec3);

#[derive(Component)]
struct BonusComponent;

enum EnemyState {
    Chasing,
    Cooldown
}

#[derive(Component)]
struct Enemy(EnemyState);

#[derive(Component)]
struct Health(i32);

#[derive(Default)]
struct Bonus {
    entity: Option<Entity>,
    x: f32,
    z: f32,
    handle: Handle<Scene>,
}

#[derive(Resource, Default)]
struct Game {
    bonus: Bonus,
    player: Option<Entity>
}

#[derive(Resource)]
struct BonusSpawnTimer(Timer);

#[derive(Resource)]
struct EnemyAttackTimer(Timer);

const DEFAULT_PLAYER_POS: [f32; 3] = [ 0.0, 1.0, 0.0];
const DEFAULT_CAMERA_POS: [f32; 3] = [-7.0, 10.0, 0.0];

fn move_camera(
    mut camera_transform: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<MainCamera>)>
) {
    let player_pos = player_transform.single_mut().translation;

    let camera_distance: Vec3 = Vec3::from(DEFAULT_CAMERA_POS) - Vec3::from(DEFAULT_PLAYER_POS);
    let new_camera_pos = player_pos + camera_distance;

    // Interpolated camera movement
    camera_transform.single_mut().translation = camera_transform.single_mut().translation.lerp(new_camera_pos, 0.2);
}

fn move_cursor(
    rapier_context: Res<RapierContext>,
    windows: ResMut<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform), (With<MainCamera>, Without<Player>)>,
    mut cursor_transform: Query<&mut Transform, With<Cursor>>
) {
    let (camera, camera_transform) = q_camera.single();
    if let Some(screen_pos) = windows.primary().cursor_position() {
        let world_ray = camera
            .viewport_to_world(camera_transform, screen_pos)
            .unwrap();
        // println!("{:?}", world_ray);

        let ray_pos = world_ray.origin;
        let ray_dir = world_ray.direction;
        let max_toi = 100.0;
        let solid = true;
        let filter = QueryFilter {
            ..default()
        };
        if let Some((_entity, intersection)) = rapier_context.cast_ray_and_get_normal(
            ray_pos, ray_dir, max_toi, solid, filter
        ) {
            let hit_point = intersection.point;
            // println!("Entity {:?} hit at point {} with normal {}", entity, hit_point, hit_normal);
            cursor_transform.single_mut().translation = hit_point;
        }
    }
}

fn move_player(
    keyboard_input: Res<Input<KeyCode>>,
    mut player_velocities: Query<&mut Velocity, With<Player>>,
    mut player_transform: Query<&mut Transform, With<Player>>,
    cursor_transform: Query<&mut Transform, (With<Cursor>, Without<Player>)>,
    game: ResMut<Game>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>
) {
    const SPEED: f32 = 250.0;
    let mut vel = player_velocities.single_mut();
    let mut transform = player_transform.single_mut();

    // Rotate character using cursor
    let x_pos = cursor_transform.single().translation.x - transform.translation.x;
    let z_pos = cursor_transform.single().translation.z - transform.translation.z;
    let angle = (x_pos).atan2(z_pos) - FRAC_PI_2;
    transform.rotation = Quat::from_rotation_y(angle);

    let mut x = 0.0;
    let mut z = 0.0;
    if keyboard_input.any_pressed([KeyCode::Up, KeyCode::W]) {
        x = 1.0;
    }
    else if keyboard_input.any_pressed([KeyCode::Down, KeyCode::S]) {
        x = -1.0;
    }
    
    if keyboard_input.any_pressed([KeyCode::Left, KeyCode::A]) {
        z = -1.0;
    }
    else if keyboard_input.any_pressed([KeyCode::Right, KeyCode::D]) {
        z = 1.0;
    }
    
    if x == 0.0 && z == 0.0 {
        vel.linvel[0] = 0.0;
        vel.linvel[2] = 0.0;
    } 
    else {
        let v2_norm = Vec2::new(x,z).normalize();
        vel.linvel[0] = v2_norm.x * SPEED * time.delta_seconds();
        vel.linvel[2] = v2_norm.y * SPEED * time.delta_seconds();
    }
    
    if keyboard_input.just_pressed(KeyCode::Space) {
        // Prevent double-jump using raycast
        let ray_pos = transform.translation;
        let ray_dir = Vec3::new(0.0, -1.0, 0.0);
        let max_toi = 1.0;
        let solid = true;
        let filter = QueryFilter {
            exclude_collider: game.player,
            ..default()
        };
    
        if let Some((_entity, _toi)) = rapier_context.cast_ray(
            ray_pos, ray_dir, max_toi, solid, filter
        ) {
            vel.linvel[1] = 6.0;
        }
    
    }

    // Custom gravity
    // vel.linvel[1] -= 1.0;
}

fn get_bonus(
    mut player: Query<(Entity, &mut Health), With<Player>>,
    bonus: Query<(Entity, &mut Transform), With<BonusComponent>>,
    mut game: ResMut<Game>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
) {
    for (bonus_entity, bonus_transform) in bonus.iter() {
        let shape = Collider::ball(0.5);
        let shape_pos = bonus_transform.translation;
        let shape_rot = bonus_transform.rotation;
        let filter = QueryFilter::default();
        
        rapier_context.intersections_with_shape(
            shape_pos, shape_rot, &shape, filter, |entity| {
            if entity == player.single().0 {
                commands.entity(bonus_entity).despawn_recursive();
                game.bonus.entity = None;
    
                // Add player health
                player.single_mut().1.0 += 1;
            }
            true
        });
    }
}

fn player_melee_attack(
    mut enemies: Query<(Entity, &mut Health), (With<Enemy>, Without<Player>, Without<Cursor>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<Enemy>, Without<Cursor>)>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        for (enemy_entity, mut enemy_health) in enemies.iter_mut() {
            let shape = Collider::ball(1.0);
            let shape_pos = player_transform.single_mut().translation;
            let shape_rot = player_transform.single_mut().rotation;
            let filter = QueryFilter::default();
            
            rapier_context.intersections_with_shape(
                shape_pos, shape_rot, &shape, filter, |entity| {
                if entity == enemy_entity {
                    enemy_health.0 -= 1;
                    if enemy_health.0 == 0 {
                        commands.entity(enemy_entity).despawn_recursive();
                    }
                }
                true
            });
        }
    }
}

fn player_shoot_attack(
    mut player: Query<&mut Transform, (With<Player>, Without<Cursor>)>,
    mut cursor_transform: Query<&mut Transform, (With<Cursor>, Without<Player>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<Input<MouseButton>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        let direction = (
            cursor_transform.single_mut().translation - player.single_mut().translation 
        ).normalize();
        
        let sphere = render_shape::Capsule {
            depth: 0.0,
            radius: 0.1,
            ..default()
        };
        commands
            .spawn(PbrBundle {
                mesh		: meshes.add(Mesh::from(sphere)),
                material	: materials.add(Color::GOLD.into()),
                transform	: Transform::from_translation(player.single_mut().translation),
                ..default()
            })
            .insert(Bullet(direction))
            .insert(RigidBody::Dynamic)
            .insert(Velocity::zero())
            // .insert(Collider::ball(0.1))
            .insert(Restitution::coefficient(0.7));
    }
}

fn move_bullets(
    mut bullet_velocities: Query<(Entity, &mut Velocity, &mut Bullet, &mut Transform), With<Bullet>>,
    mut enemies: Query<(Entity, &mut Health), With<Enemy>>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    game: ResMut<Game>,
) {
    const SPEED: f32 = 10.0;
    for (bullet_entity, mut vel, direction, transform) in bullet_velocities.iter_mut() {
        vel.linvel[0] = direction.0.x * SPEED;
        vel.linvel[2] = direction.0.z * SPEED;

        let shape = Collider::ball(0.1);
        let shape_pos = transform.translation;
        let shape_rot = transform.rotation;
        let shape_vel = vel.linvel;
        let max_toi = 0.0;
        let filter = QueryFilter {
            exclude_collider: game.player,
            ..default()
        };
        
        if let Some((entity, _hit)) = rapier_context.cast_shape(
            shape_pos, shape_rot, shape_vel, &shape, max_toi, filter
        ) {
            // Despawn bullet after it hits anything
            commands.entity(bullet_entity).despawn_recursive();

            for (enemy_entity, mut enemy_health) in enemies.iter_mut() {
                if entity == enemy_entity {
                    enemy_health.0 -= 1;
                    if enemy_health.0 == 0 {
                        commands.entity(enemy_entity).despawn_recursive();
                    }
                }
            }
        }
    }
}

fn enemy_attack(
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
    mut enemies: Query<(&mut Transform, &mut Enemy), (With<Enemy>, Without<Player>)>,
    mut player: Query<(&mut Transform, &mut Health), (With<Player>, Without<Enemy>)>,
) {    
    let vec2_player = Vec2::new(player.single_mut().0.translation[0], player.single_mut().0.translation[2]);
    for (enemy_transform, mut enemy_state) in enemies.iter_mut() {
        let vec2_enemy = Vec2::new(enemy_transform.translation[0], enemy_transform.translation[2]);
        
        if vec2_player.distance(vec2_enemy) < 1.5 {
            match enemy_state.0 {
                EnemyState::Chasing => {
                    // Attack player
                    player.single_mut().1.0 -= 1;
                    enemy_state.0 = EnemyState::Cooldown;
                },
                _ => {
                    // Enemy attack cooldown
                    if !timer.0.tick(time.delta()).finished() {
                        return;
                    }
                    enemy_state.0 = EnemyState::Chasing;
                }
            }
        }
    }
}

fn move_enemy(
    mut enemies: Query<(&mut Transform, &mut Velocity), (With<Enemy>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<Enemy>)>
) {
    const SPEED: f32 = 6.0;
    for (mut enemy_transform, mut enemy_velocity) in enemies.iter_mut() {
        // Get vector representing direction from enemy to player
        let mut direction_vec = player_transform.single_mut().translation - enemy_transform.translation;
        direction_vec = direction_vec.normalize();

        // Calculate distance of vectors, so enemy chases player only until it's near him
        let vec2_player = Vec2::new(player_transform.single_mut().translation[0], player_transform.single_mut().translation[2]);
        let vec2_enemy = Vec2::new(enemy_transform.translation[0], enemy_transform.translation[2]);
        if vec2_player.distance(vec2_enemy) > 1.4 {
            enemy_velocity.linvel[0] = direction_vec[0] * SPEED;
            enemy_velocity.linvel[2] = direction_vec[2] * SPEED;
        }

        // Rotate ememy in direction it's moving
        let dir_angle = (direction_vec.x).atan2(direction_vec.z);
        enemy_transform.rotation = Quat::from_rotation_y(dir_angle);   
    } 
}

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::from(DEFAULT_CAMERA_POS))
                .looking_at(Vec3::from(DEFAULT_PLAYER_POS), Vec3::Y), // focus rotation of camera on player
            ..default()
        })
        .insert(MainCamera);
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
        
    // Spawn alien
    commands
        .spawn(Enemy(EnemyState::Chasing))
        .insert(Health(3))
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

    // Spawn skeleton
    commands
        .spawn(Enemy(EnemyState::Chasing))
        .insert(Health(3))
        .insert(PbrBundle {
            transform: Transform::from_xyz(5.0, 1.0, -5.0),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn(SceneBundle {
                scene: asset_server.load("models/AlienCake/characterSkeleton.glb#Scene0"), 
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
    mut game: ResMut<Game>,
) {
    game.player = Some(
        commands
            .spawn(Player)
            .insert(Health(5))
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
            .insert(Restitution::coefficient(0.7))
            .id()
    )
}

// despawn the bonus if there is one, then spawn a new one at a random location
fn spawn_bonus(
    time: Res<Time>,
    mut timer: ResMut<BonusSpawnTimer>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut player_transform: Query<&mut Transform, With<Player>>
) {
    // Make sure we wait enough time before spawning the next bonus
    if !timer.0.tick(time.delta()).finished() {
        return;
    }

    if let Some(entity) = game.bonus.entity {
        commands.entity(entity).despawn_recursive();
        game.bonus.entity = None;
    }

    // Ensure bonus doesn't spawn on the player
    loop {
        game.bonus.x = rand::thread_rng().gen_range(-7.0..7.0);
        game.bonus.z = rand::thread_rng().gen_range(-7.0..7.0);
        let player_pos = Vec2::new(player_transform.single_mut().translation[0], player_transform.single_mut().translation[2]);
        let bonus_pos = Vec2::new(game.bonus.x, game.bonus.z);
        if player_pos.distance(bonus_pos) > 2.0 {
            break;
        }
    }
    game.bonus.entity = Some(
        commands
            .spawn(SceneBundle {
                transform: Transform {
                    translation: Vec3::new(game.bonus.x, 0.5, game.bonus.z),
                    scale:  Vec3::new(2.0, 2.0, 2.0),
                    ..default()
                },
                scene: game.bonus.handle.clone(),
                ..default()
            })
            .with_children(|children| {
                children.spawn(PointLightBundle {
                    point_light: PointLight {
                        color: Color::rgb(1.0, 1.0, 0.0),
                        intensity: 1000.0,
                        range: 10.0,
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 2.0, 0.0),
                    ..default()
                });
            })
            .insert(Velocity::zero())
            .insert(BonusComponent)
            .id(),
    );
}

fn setup_physics(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    game: ResMut<Game>,
) {
    spawn_level(&asset_server, &mut meshes, &mut materials, &mut commands);
    spawn_player(&asset_server, &mut commands, game);
}

fn setup(
    asset_server: Res<AssetServer>, 
    mut game: ResMut<Game>, 
    mut commands: Commands,
) {
    // load the scene for the bonus
    game.bonus.handle = asset_server.load("models/AlienCake/pumpkin.glb#Scene0");

    // scoreboard
    commands.spawn(
        TextBundle::from_section(
            "Score:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.5, 0.5, 1.0),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
            ..default()
        }),
    );

    // Setup cursor
    commands
        .spawn(SceneBundle {
            transform: Transform {
                translation: Vec3::ZERO,
                scale:  Vec3::new(2.0, 2.0, 2.0),
                ..default()
            },
            // scene: game.bonus.handle.clone(),
            ..default()
        })
        .insert(Cursor);
}

// Update the health displayed during the game
fn show_health(
    mut text_query: Query<&mut Text>,
    mut health_query: Query<&mut Health, With<Player>>
) {
    let mut text = text_query.single_mut();
    let health = health_query.single_mut();
    text.sections[0].value = format!("Health: {}", health.0);
}

fn main() {
    App::new()
        .init_resource::<Game>()
        .insert_resource(BonusSpawnTimer(Timer::from_seconds(
            5.0,
            TimerMode::Repeating,
        )))
        .insert_resource(EnemyAttackTimer(Timer::from_seconds(
            2.0,
            TimerMode::Repeating,
        )))
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_state(GameState::Playing)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_light)
        .add_startup_system(setup_physics)
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(move_player)
                .with_system(move_cursor)
                .with_system(move_camera)
                .with_system(move_enemy)
                .with_system(spawn_bonus)
                .with_system(player_melee_attack)
                .with_system(player_shoot_attack)
                .with_system(show_health)
                .with_system(enemy_attack)
                .with_system(move_bullets)
                .with_system(get_bonus)
        )
        .add_system(bevy::window::close_on_esc)
        .run();
}