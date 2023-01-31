use std::f32::consts::FRAC_PI_2;

use bevy::{ecs::schedule::SystemSet, prelude::*};
use bevy::render::mesh::shape as render_shape;
use bevy_rapier3d::prelude::*;

use crate::{GameState, Health, Game, Cursor};
use crate::enemies::Enemy;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct Bullet {
    direction: Vec3,
    start_position: Vec3
}

#[derive(PartialEq)]
enum WeaponType {
    Pistol,
    Shotgun,
    Rifle
}

#[derive(Component)]
pub struct Weapon(WeaponType);

#[derive(Resource)]
struct RifleCooldownTimer(Timer);

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RifleCooldownTimer(Timer::from_seconds(
                0.1,
                TimerMode::Repeating,
            )))    
            .add_startup_system(setup)
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(move_player)
                    .with_system(player_melee_attack)
                    .with_system(player_shoot_attack)
                    .with_system(move_player_bullets)
                    .with_system(change_weapon)
            );
    }
}

fn setup(
    asset_server: Res<AssetServer>, 
    mut commands: Commands, 
    mut game: ResMut<Game>
) {
    game.player = Some(
        commands
            .spawn(Player)
            .insert(Health(5))
            .insert(Weapon(WeaponType::Pistol))
            .insert(PbrBundle {
                transform: Transform::from_xyz(0.0, 1.0, 0.0),
                ..default()
            })
            .with_children(|cell| {
                cell.spawn(SceneBundle {
                    scene: asset_server.load("models/characterDigger.glb#Scene0"),
                    transform: Transform {
                        translation: Vec3::new(0.0, -1.0, 0.0),
                        rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                        scale: Vec3::new(2.0, 2.0, 2.0),
                    },
                    ..default()
                });
            })
            .insert(RigidBody::Dynamic)
            .insert(Velocity::zero())
            .insert(LockedAxes::ROTATION_LOCKED)
            // .insert(GravityScale(0.0)) // May be needed when tweaking movement for RigidBody::Dynamic
            .insert(Collider::capsule_y(0.5, 0.5))
            .insert(Restitution::coefficient(0.7))
            .id(),
    )
}

fn move_player(
    keyboard_input: Res<Input<KeyCode>>,
    mut player: Query<(&mut Velocity, &mut Transform), With<Player>>,
    cursor_transform: Query<&Transform, (With<Cursor>, Without<Player>)>,
    game: ResMut<Game>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
) {
    const SPEED: f32 = 250.0;
    let (mut vel, mut transform) = player.single_mut();

    // Rotate character using cursor
    let x_pos = cursor_transform.single().translation.x - transform.translation.x;
    let z_pos = cursor_transform.single().translation.z - transform.translation.z;
    let angle = (x_pos).atan2(z_pos) - FRAC_PI_2;
    transform.rotation = Quat::from_rotation_y(angle);

    let mut x = 0.0;
    let mut z = 0.0;
    if keyboard_input.any_pressed([KeyCode::Up, KeyCode::W]) {
        x = 1.0;
    } else if keyboard_input.any_pressed([KeyCode::Down, KeyCode::S]) {
        x = -1.0;
    }

    if keyboard_input.any_pressed([KeyCode::Left, KeyCode::A]) {
        z = -1.0;
    } else if keyboard_input.any_pressed([KeyCode::Right, KeyCode::D]) {
        z = 1.0;
    }

    if x == 0.0 && z == 0.0 {
        vel.linvel[0] = 0.0;
        vel.linvel[2] = 0.0;
    } else {
        let v2_norm = Vec2::new(x, z).normalize();
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

        if let Some((_entity, _toi)) =
            rapier_context.cast_ray(ray_pos, ray_dir, max_toi, solid, filter)
        {
            vel.linvel[1] = 6.0;
        }
    }

    // Custom gravity
    // vel.linvel[1] -= 1.0;
}

fn player_melee_attack(
    mut enemies: Query<(Entity, &mut Health), (Without<Player>, Without<Cursor>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<Cursor>)>,
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
                shape_pos,
                shape_rot,
                &shape,
                filter,
                |entity| {
                    if entity == enemy_entity {
                        enemy_health.0 -= 1;
                        if enemy_health.0 == 0 {
                            commands.entity(enemy_entity).despawn_recursive();
                        }
                    }
                    true
                },
            );
        }
    }
}

fn spawn_bullet(
    origin: Vec3,
    direction: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    commands: &mut Commands,
) {
    let sphere = render_shape::Capsule {
        depth: 0.0,
        radius: 0.1,
        ..default()
    };
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(sphere)),
            material: materials.add(Color::GOLD.into()),
            transform: Transform::from_translation(origin),
            ..default()
        })
        .insert(Bullet {
            direction,
            start_position: origin
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero());
}

fn player_shoot_attack(
    mut player: Query<(&Transform, &Weapon), (With<Player>, Without<Cursor>)>,
    mut cursor_transform: Query<&Transform, (With<Cursor>, Without<Player>)>,
    mouse: Res<Input<MouseButton>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<RifleCooldownTimer>,
) {
    let mut direction: Vec3 = cursor_transform.single_mut().translation - player.single_mut().0.translation;
    let direction_vec2 = Vec2::new(direction.x, direction.z).normalize();
    direction = Vec3::new(direction_vec2.x, 0.0, direction_vec2.y);

    if mouse.just_pressed(MouseButton::Left) {
        match player.single_mut().1.0 {
            WeaponType::Pistol => {
                spawn_bullet(player.single_mut().0.translation, direction, &mut meshes, &mut materials, &mut commands);
            },
            WeaponType::Shotgun => {
                spawn_bullet(player.single_mut().0.translation, Quat::from_rotation_y(-0.6) * direction, &mut meshes, &mut materials, &mut commands);
                spawn_bullet(player.single_mut().0.translation, Quat::from_rotation_y(-0.3) * direction, &mut meshes, &mut materials, &mut commands);
                spawn_bullet(player.single_mut().0.translation, direction, &mut meshes, &mut materials, &mut commands);
                spawn_bullet(player.single_mut().0.translation, Quat::from_rotation_y(0.3) * direction, &mut meshes, &mut materials, &mut commands);
                spawn_bullet(player.single_mut().0.translation, Quat::from_rotation_y(0.6) * direction, &mut meshes, &mut materials, &mut commands);
            },
            _ => {}
        }     
    }

    if mouse.pressed(MouseButton::Left) {
        if !timer.0.tick(time.delta()).finished() {
            return;
        }
        match player.single_mut().1.0 {
            WeaponType::Rifle => {
                spawn_bullet(player.single_mut().0.translation, direction, &mut meshes, &mut materials, &mut commands);
            },
            _ => {}
        }
    }
}

fn move_player_bullets(
    mut bullets: Query<
        (Entity, &mut Velocity, &Bullet, &Transform),
        With<Bullet>,
    >,
    mut enemies: Query<(Entity, &mut Health), With<Enemy>>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    game: ResMut<Game>,
) {
    const SPEED: f32 = 15.0;
    for (bullet_entity, mut vel, bullet_struct, transform) in bullets.iter_mut() {
        // Despawn bullet after certain distance traveled
        if bullet_struct.start_position.distance(transform.translation) > 20.0 {
            commands.entity(bullet_entity).despawn_recursive();
        }

        vel.linvel[0] = bullet_struct.direction.x * SPEED;
        vel.linvel[2] = bullet_struct.direction.z * SPEED;

        let shape = Collider::ball(0.1);
        let shape_pos = transform.translation;
        let shape_rot = transform.rotation;
        let shape_vel = vel.linvel;
        let max_toi = 0.0;
        let filter = QueryFilter {
            exclude_collider: game.player,
            ..default()
        };

        if let Some((entity, _hit)) =
            rapier_context.cast_shape(shape_pos, shape_rot, shape_vel, &shape, max_toi, filter)
        {
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

fn change_weapon(
    mut mouse_wheel_er: EventReader<bevy::input::mouse::MouseWheel>,
    mut player_weapon: Query<&mut Weapon, With<Player>>,
) {
    for event in mouse_wheel_er.iter() {
        if event.y == -1.0 {
            player_weapon.single_mut().0 = match player_weapon.single_mut().0 {
                WeaponType::Pistol => WeaponType::Shotgun, 
                WeaponType::Shotgun => WeaponType::Rifle, 
                WeaponType::Rifle => WeaponType::Pistol, 
            }
        }
    }
}