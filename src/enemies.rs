use std::f32::consts::PI;
use rand::Rng;

use bevy::render::mesh::shape as render_shape;
use bevy::{ecs::schedule::SystemSet, prelude::*};
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::{GameState, Health};

#[derive(Component)]
struct EnemyBullet{
    shooter: Entity,
    direction: Vec3,
    start_position: Vec3
}

pub enum ChasingEnemyState {
    Chasing,
    Cooldown,
}

pub enum ShootingEnemyState {
    Shooting,
    Cooldown,
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct ChasingEnemy(pub ChasingEnemyState);

#[derive(Component)]
pub struct ShootingEnemy(pub ShootingEnemyState);

#[derive(Resource)]
struct EnemyAttackTimer(Timer);

#[derive(Resource)]
struct EnemySpawnTimer(Timer);

// #[derive(Resource)]
// struct Animations(Vec<Handle<AnimationClip>>);

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
        // .add_startup_system(setup)
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(spawn_enemies)
                .with_system(rotate_enemy)
                .with_system(move_enemy)
                .with_system(enemy_melee_attack)
                .with_system(enemy_shoot_attack)
                .with_system(move_enemy_bullets)
                // .with_system(animate_enemies)
        );
    }
}

// fn animate_enemies(
//     animations: Res<Animations>,
//     mut anim_player: Query<&mut AnimationPlayer>,
//     enemies: Query<&Velocity, With<Enemy>>,
// ) {
//     if let Ok(mut anim_player) = anim_player.get_single_mut() {
//         for enemy_velocity in enemies.iter() {
//             if enemy_velocity.linvel.length() < 0.5 {
//                 anim_player.play(animations.0[0].clone_weak()).repeat();
//             }
//             else {
//                 anim_player.play(animations.0[2].clone_weak()).repeat();
//             }
//         }
//     }
// }

// fn setup(
//     asset_server: Res<AssetServer>, 
//     mut commands: Commands, 
// ) {
//     commands.insert_resource(Animations(vec![
//         asset_server.load("models/enemy1_anim.glb#Animation0"),
//         asset_server.load("models/enemy1_anim.glb#Animation1"),
//         asset_server.load("models/enemy1_anim.glb#Animation2")
//     ]));
// }

fn spawn_enemies(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
) {
    if timer.0.tick(time.delta()).elapsed_secs() != 0.0 && !timer.0.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();

    // Spawn alien
    for _ in 0..rng.gen_range(1..=2) {
        let angle: f32 = rng.gen_range(0.0..1.0) * PI * 2.0;
        let x_alien = angle.sin() * 7.0;
        let z_alien = angle.cos() * 7.0;
        commands
            .spawn(Enemy)
            .insert(ChasingEnemy(ChasingEnemyState::Chasing))
            .insert(Health(3))
            .insert(PbrBundle {
                transform: Transform::from_xyz(x_alien, 1.0, z_alien),
                ..default()
            })
            .with_children(|cell| {
                cell.spawn(SceneBundle {
                    scene: asset_server.load("models/alien.glb#Scene0"),
                    // scene: asset_server.load("models/enemy1_anim.glb#Scene0"),
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

    // Spawn skeletons
    for _ in 0..rng.gen_range(1..=1) {
        let angle: f32 = rng.gen_range(0.0..1.0) * PI * 2.0;
        let x_skeleton = angle.sin() * 7.0;
        let z_skeleton = angle.cos() * 7.0;
        commands
            .spawn(Enemy)
            .insert(ShootingEnemy(ShootingEnemyState::Shooting))
            .insert(Health(3))
            .insert(PbrBundle {
                transform: Transform::from_xyz(x_skeleton, 1.0, z_skeleton),
                ..default()
            })
            .with_children(|cell| {
                cell.spawn(SceneBundle {
                    scene: asset_server.load("models/skeleton.glb#Scene0"),
                    // scene: asset_server.load("models/enemy2_anim.glb#Scene0"),
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
}

fn move_enemy_bullets(
    mut bullets: Query<
        (Entity, &mut Velocity, &EnemyBullet, &Transform),
        With<EnemyBullet>,
    >,
    mut player: Query<(Entity, &mut Health), With<Player>>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
) {
    const SPEED: f32 = 10.0;
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

fn enemy_shoot_attack(
    mut player: Query<(&Transform, &mut Health), (With<Player>, Without<ShootingEnemy>)>,
    mut enemies: Query<
        (Entity, &Transform, &mut ShootingEnemy),
        (With<ShootingEnemy>, Without<Player>),
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
) {
    for (enemy_entity, enemy_transform, mut enemy_state) in enemies.iter_mut() {
        let direction =
            (player.single_mut().0.translation - enemy_transform.translation).normalize();

        match enemy_state.0 {
            ShootingEnemyState::Shooting => {
                // Shoot bullet
                let sphere = render_shape::Capsule {
                    depth: 0.0,
                    radius: 0.1,
                    ..default()
                };
                commands
                    .spawn(PbrBundle {
                        mesh: meshes.add(Mesh::from(sphere)),
                        material: materials.add(Color::BLUE.into()),
                        transform: Transform::from_translation(enemy_transform.translation),
                        ..default()
                    })
                    .insert(EnemyBullet{
                        shooter: enemy_entity,
                        direction,
                        start_position: enemy_transform.translation
                    })
                    .insert(RigidBody::Dynamic)
                    .insert(Velocity::zero());

                enemy_state.0 = ShootingEnemyState::Cooldown;
            }
            _ => {
                // Enemy attack cooldown
                if !timer.0.tick(time.delta()).finished() {
                    continue;
                }
                enemy_state.0 = ShootingEnemyState::Shooting;
            }
        }
    }
}

fn enemy_melee_attack(
    mut enemies: Query<(&Transform, &mut ChasingEnemy), With<Enemy>>,
    mut player: Query<(Entity, &mut Health), With<Player>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
) {
    for (enemy_transform, mut enemy_state) in enemies.iter_mut() {
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
            match enemy_state.0 {
                ChasingEnemyState::Chasing => {
                    // Attack player
                    player.single_mut().1 .0 -= 1;
                    enemy_state.0 = ChasingEnemyState::Cooldown;
                }
                _ => {
                    // Enemy attack cooldown
                    if !timer.0.tick(time.delta()).finished() {
                        continue;
                    }
                    enemy_state.0 = ChasingEnemyState::Chasing;
                }
            }
        }
    }
}

fn rotate_enemy(
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

fn move_enemy(
    mut enemies: Query<(&Transform, &mut Velocity), (With<ChasingEnemy>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<ChasingEnemy>)>,
) {
    const SPEED: f32 = 6.0;
    for (enemy_transform, mut enemy_velocity) in enemies.iter_mut() {
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
            enemy_velocity.linvel[0] = direction_vec[0] * SPEED;
            enemy_velocity.linvel[2] = direction_vec[2] * SPEED;
        }
    }
}
