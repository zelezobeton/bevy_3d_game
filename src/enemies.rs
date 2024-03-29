use std::f32::consts::PI;
use rand::Rng;

use bevy::render::mesh::shape as render_shape;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::{GameState, Health, FloatingTextEvent};

#[derive(Component)]
struct EnemyBullet{
    shooter: Entity,
    direction: Vec3,
    start_position: Vec3
}

pub enum EnemyState {
    Attacking,
    Cooldown,
}

#[derive(PartialEq)]
pub enum EnemyType {
    Chasing,
    Pistol,
    Shotgun,
    Star
}

#[derive(Component)]
pub struct Enemy {
    enemy_type: EnemyType,
    enemy_state: EnemyState
}

#[derive(Resource)]
struct EnemyAttackTimer(Timer);

#[derive(Resource)]
struct EnemySpawnTimer(Timer);

pub struct EnemiesPlugin;
impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemyAttackTimer(Timer::from_seconds(
            2.0,
            TimerMode::Repeating,
        )))
        .insert_resource(EnemySpawnTimer(Timer::from_seconds(
            7.0,
            TimerMode::Repeating,
        )))
        .add_systems(
            Update,
            (
                spawn_enemies,
                rotate_enemies,
                move_enemies,
                enemy_melee_attack,
                enemy_shoot_attack,
                move_enemy_bullets
            )
            .in_set(GameState::Playing),
        );
    }
}

fn spawn_enemies(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
) { 
    if !timer.0.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    match rng.gen_range(1..=4) {
        1 => {
            spawn_enemy(EnemyType::Chasing, "models/characterZombie.glb#Scene0", &mut commands, &asset_server)
        },
        2 => {
            spawn_enemy(EnemyType::Pistol, "models/characterSkeleton.glb#Scene0", &mut commands, &asset_server)
        }
        3 => {
            spawn_enemy(EnemyType::Shotgun, "models/characterGhost.glb#Scene0", &mut commands, &asset_server)
        },
        4 => {
            spawn_enemy(EnemyType::Star, "models/characterVampire.glb#Scene0", &mut commands, &asset_server)
        },
        _ => unreachable!()
    }
}

fn spawn_enemy(
    enemy_type: EnemyType,
    model: &str,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();
    let angle: f32 = rng.gen_range(0.0..1.0) * PI * 2.0;
    let x = angle.sin() * 7.0;
    let z = angle.cos() * 7.0;
    commands
        .spawn(Enemy{enemy_state: EnemyState::Attacking, enemy_type: enemy_type})
        .insert(Health(3))
        .insert(PbrBundle {
            transform: Transform::from_xyz(x, 1.0, z),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn(SceneBundle {
                scene: asset_server.load(model),
                transform: Transform {
                    translation: Vec3::new(0.0, -1.0, 0.0),
                    rotation: Quat::from_rotation_y(PI),
                    scale: Vec3::new(2.0, 2.0, 2.0),
                },
                ..default()
            });
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero())
        .insert(Collider::capsule_y(0.5, 0.5));
}

fn enemy_shoot_attack(
    mut player: Query<(&Transform, &mut Health), (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (Entity, &Transform, &mut Enemy),
        (With<Enemy>, Without<Player>),
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
) {
    for (enemy_entity, enemy_transform, mut enemy) in enemies.iter_mut() {
        if enemy.enemy_type == EnemyType::Chasing {
            continue
        }

        let direction =
            (player.single_mut().0.translation - enemy_transform.translation).normalize();

        match enemy.enemy_state {
            EnemyState::Attacking => {

                match enemy.enemy_type {
                    EnemyType::Pistol => {
                        spawn_bullet(enemy_transform.translation, direction, enemy_entity, &mut meshes, &mut materials, &mut commands);
                    }
                    EnemyType::Shotgun => {
                        spawn_bullet(enemy_transform.translation, Quat::from_rotation_y(-0.3) * direction, enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, direction, enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Quat::from_rotation_y(0.3) * direction, enemy_entity, &mut meshes, &mut materials, &mut commands);
                    }
                    EnemyType::Star => {
                        spawn_bullet(enemy_transform.translation, Vec3::new(1.0, 0.0, 0.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(0.0, 0.0, 1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(0.0, 0.0, -1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(-1.0, 0.0, 0.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(1.0, 0.0, 1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(-1.0, 0.0, 1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(1.0, 0.0, -1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        spawn_bullet(enemy_transform.translation, Vec3::new(-1.0, 0.0, -1.0), enemy_entity, &mut meshes, &mut materials, &mut commands);
                        
                    }
                    _ => {}
                }

                enemy.enemy_state = EnemyState::Cooldown;
            }
            _ => {
                // Enemy attack cooldown
                if !timer.0.tick(time.delta()).finished() {
                    continue;
                }
                enemy.enemy_state = EnemyState::Attacking;
            }
        }
    }
}

fn spawn_bullet(
    origin: Vec3,
    direction: Vec3,
    shooter: Entity,
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
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_translation(origin),
            ..default()
        })
        .insert(EnemyBullet{
            shooter,
            direction,
            start_position: origin
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero());
}

fn move_enemy_bullets(
    mut bullets: Query<
        (Entity, &mut Velocity, &EnemyBullet, &Transform),
        With<EnemyBullet>,
    >,
    mut player: Query<(Entity, &mut Health), With<Player>>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    time: Res<Time>,
) {
    const SPEED: f32 = 600.0;
    for (bullet_entity, mut vel, bullet_struct, transform) in bullets.iter_mut() {
        // Despawn bullet after certain distance traveled
        if bullet_struct.start_position.distance(transform.translation) > 20.0 {
            commands.entity(bullet_entity).despawn_recursive();
        }

        vel.linvel[0] = bullet_struct.direction.x * SPEED * time.delta_seconds();
        vel.linvel[2] = bullet_struct.direction.z * SPEED * time.delta_seconds();

        let shape = Collider::ball(0.1);
        let shape_pos = transform.translation;
        let shape_rot = transform.rotation;
        let shape_vel = vel.linvel;
        let max_toi = 0.0;
        let filter = QueryFilter {
            exclude_collider: Some(bullet_struct.shooter),
            ..default()
        };

        if let Some((entity, _hit)) =
            rapier_context.cast_shape(shape_pos, shape_rot, shape_vel, &shape, max_toi, filter)
        {
            // Despawn bullet after it hits anything
            commands.entity(bullet_entity).despawn_recursive();

            if entity == player.single().0 {
                player.single_mut().1 .0 -= 1;
            }
        }
    }
}

fn enemy_melee_attack(
    mut enemies: Query<(&Transform, &mut Enemy), With<Enemy>>,
    mut player: Query<(Entity, &mut Health, &Transform), With<Player>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
    mut floating_text_event_writer: EventWriter<FloatingTextEvent>,
) {
    for (enemy_transform, mut enemy) in enemies.iter_mut() {
        if enemy.enemy_type != EnemyType::Chasing {
            continue
        }

        let shape = Collider::ball(1.0);
        let shape_pos = enemy_transform.translation;
        let shape_rot = enemy_transform.rotation;
        let filter = QueryFilter::default();

        let mut player_is_hit = false;
        rapier_context.intersections_with_shape(shape_pos, shape_rot, &shape, filter, |entity| {
            if entity == player.single().0 {
                player_is_hit = true;
            }
            true
        });

        if player_is_hit {
            match enemy.enemy_state {
                EnemyState::Attacking => {
                    // Attack player
                    player.single_mut().1.0 -= 1;

                    // Create floating text
                    floating_text_event_writer.send(FloatingTextEvent {
                        translation: player.single_mut().2.translation,
                        text: "-1".into(),
                        color: Color::rgb(0.7, 0.0, 0.0),
                    });

                    enemy.enemy_state = EnemyState::Cooldown;
                }
                _ => {
                    // Enemy attack cooldown
                    if !timer.0.tick(time.delta()).finished() {
                        continue;
                    }
                    enemy.enemy_state = EnemyState::Attacking;
                }
            }
        }
    }
}

fn rotate_enemies(
    mut enemies: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    for mut enemy_transform in enemies.iter_mut() {
        // Get vector representing direction from enemy to player
        let mut direction_vec =
            player_transform.single_mut().translation - enemy_transform.translation;
        direction_vec = direction_vec.normalize();

        // Rotate ememy in direction of player
        let dir_angle = (direction_vec.x).atan2(direction_vec.z);
        enemy_transform.rotation = Quat::from_rotation_y(dir_angle);
    }
}

fn move_enemies(
    mut enemies: Query<(&Transform, &mut Velocity, &mut Enemy), (With<Enemy>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<Enemy>)>,
    time: Res<Time>,
) {
    const SPEED: f32 = 200.0;
    for (enemy_transform, mut enemy_velocity, enemy) in enemies.iter_mut() {
        if enemy.enemy_type != EnemyType::Chasing {
            continue
        }

        // Get vector representing direction from enemy to player
        let mut direction_vec =
            player_transform.single_mut().translation - enemy_transform.translation;
        direction_vec = direction_vec.normalize();

        // Calculate distance of vectors, so enemy chases player only until it's near him
        let vec2_player = Vec2::new(
            player_transform.single_mut().translation[0],
            player_transform.single_mut().translation[2],
        );
        let vec2_enemy = Vec2::new(
            enemy_transform.translation[0],
            enemy_transform.translation[2],
        );
        if vec2_player.distance(vec2_enemy) > 1.4 {
            enemy_velocity.linvel[0] = direction_vec[0] * SPEED * time.delta_seconds();
            enemy_velocity.linvel[2] = direction_vec[2] * SPEED * time.delta_seconds();
        }
    }
}
