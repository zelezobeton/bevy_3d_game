use bevy::{ecs::schedule::SystemSet, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy::render::mesh::shape as render_shape;

use crate::{GameState, Health};
use crate::player::Player;

#[derive(Component)]
struct EnemyBullet(Vec3, Entity);

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

pub struct EnemiesPlugin;
impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(EnemyAttackTimer(Timer::from_seconds(
                2.0,
                TimerMode::Repeating,
            )))
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(rotate_enemy)
                    .with_system(move_enemy)
                    .with_system(enemy_melee_attack)
                    .with_system(enemy_shoot_attack)
                    .with_system(move_enemy_bullets)
            );
    }
}

fn move_enemy_bullets(
    mut bullets: Query<
        (Entity, &mut Velocity, &mut EnemyBullet, &mut Transform),
        With<EnemyBullet>,
    >,
    mut player: Query<(Entity, &mut Health), With<Player>>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
) {
    const SPEED: f32 = 10.0;
    for (bullet_entity, mut vel, bullet_tuple, transform) in bullets.iter_mut() {
        vel.linvel[0] = bullet_tuple.0.x  * SPEED;
        vel.linvel[2] = bullet_tuple.0.z * SPEED;

        let shape = Collider::ball(0.1);
        let shape_pos = transform.translation;
        let shape_rot = transform.rotation;
        let shape_vel = vel.linvel;
        let max_toi = 0.0;
        let filter = QueryFilter {
            exclude_collider: Some(bullet_tuple.1),
            ..default()
        };

        if let Some((entity, _hit)) =
            rapier_context.cast_shape(shape_pos, shape_rot, shape_vel, &shape, max_toi, filter)
        {
            // Despawn bullet after it hits anything
            commands.entity(bullet_entity).despawn_recursive();

            if entity == player.single().0 {
                player.single_mut().1.0 -= 1;
            }
        }
    }
}

fn enemy_shoot_attack(
    mut player: Query<(&mut Transform, &mut Health), (With<Player>, Without<ShootingEnemy>)>,
    mut enemies: Query<(Entity, &mut Transform, &mut ShootingEnemy), (With<ShootingEnemy>, Without<Player>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<EnemyAttackTimer>,
) {
    for (enemy_entity, enemy_transform, mut enemy_state) in enemies.iter_mut() {
        let direction = (player.single_mut().0.translation
            - enemy_transform.translation)
            .normalize();

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
                    .insert(EnemyBullet(direction, enemy_entity))
                    .insert(RigidBody::Dynamic)
                    .insert(Velocity::zero());

                enemy_state.0 = ShootingEnemyState::Cooldown;
            }
            _ => {
                // Enemy attack cooldown
                if !timer.0.tick(time.delta()).finished() {
                    return;
                }
                enemy_state.0 = ShootingEnemyState::Shooting;
            }
        }
            

    }
}

fn enemy_melee_attack(
    mut enemies: Query<(&mut Transform, &mut ChasingEnemy), With<Enemy>>,
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
                        return;
                    }
                    enemy_state.0 = ChasingEnemyState::Chasing;
                }
            }
        }
    }
}

fn rotate_enemy(
    mut enemies: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<Enemy>)>, 
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
    mut enemies: Query<(&mut Transform, &mut Velocity), (With<ChasingEnemy>, Without<Player>)>,
    mut player_transform: Query<&mut Transform, (With<Player>, Without<ChasingEnemy>)>,
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