use std::f32::consts::PI;
use rand::Rng;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy::render::mesh::shape as render_shape;

use crate::player::Player;
use crate::{GameState, Health, FloatingTextEvent};

#[derive(Component)]
struct BossBullet{
    shooter: Entity,
    direction: Vec3,
    start_position: Vec3
}

pub enum BossState {
    Attacking,
    Cooldown,
}

#[derive(PartialEq)]
pub enum BossType {
    Boss1,
    Boss2
}

#[derive(Component)]
pub struct Boss {
    boss_type: BossType,
    boss_state: BossState
}

#[derive(Resource)]
struct BossSpawnTimer(Timer);

#[derive(Resource)]
struct BossAttackTimer(Timer);

pub struct BossesPlugin;
impl Plugin for BossesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BossAttackTimer(Timer::from_seconds(
            2.0,
            TimerMode::Repeating,
        )))
        .insert_resource(BossSpawnTimer(Timer::from_seconds(
            20.0,
            TimerMode::Repeating,
        )))
        .add_systems(
            (
                spawn_bosses,
                rotate_boss,
                boss_shoot_attack,
                move_boss_bullets,
                move_bosses,
                boss_melee_attack
            )
            .in_set(OnUpdate(GameState::Playing)),
        );
    }
}

fn spawn_bosses(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<BossSpawnTimer>,
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    match rng.gen_range(1..=2) {
        1 => {
            spawn_boss(BossType::Boss1, "models/characterAlien.glb#Scene0", &mut commands, &asset_server)
        },
        2 => {
            spawn_boss(BossType::Boss2, "models/characterSkeleton.glb#Scene0", &mut commands, &asset_server)
        },
        _ => unreachable!()
    }
}

fn spawn_boss(
    boss_type: BossType,
    model: &str,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();
    let angle: f32 = rng.gen_range(0.0..1.0) * PI * 2.0;
    let x = angle.sin() * 7.0;
    let z = angle.cos() * 7.0;
    commands
        .spawn(Boss{boss_state: BossState::Attacking, boss_type: boss_type})
        .insert(Health(10))
        .insert(PbrBundle {
            transform: Transform::from_xyz(x, 1.0, z),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn(SceneBundle {
                scene: asset_server.load(model),
                transform: Transform {
                    translation: Vec3::new(0.0, -2.0, 0.0),
                    rotation: Quat::from_rotation_y(PI),
                    scale: Vec3::new(4.0, 4.0, 4.0),
                },
                ..default()
            });
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero())
        .insert(Collider::capsule_y(1.0, 1.0));
}

fn rotate_boss(
    mut bosses: Query<&mut Transform, (With<Boss>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    for mut enemy_transform in bosses.iter_mut() {
        // Get vector representing direction from enemy to player
        let mut direction_vec =
            player_transform.single_mut().translation - enemy_transform.translation;
        direction_vec = direction_vec.normalize();

        // Rotate ememy in direction of player
        let dir_angle = (direction_vec.x).atan2(direction_vec.z);
        enemy_transform.rotation = Quat::from_rotation_y(dir_angle);
    }
}

fn move_boss_bullets(
    mut bullets: Query<
        (Entity, &mut Velocity, &BossBullet, &Transform),
        With<BossBullet>,
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

fn move_bosses(
    mut bosses: Query<(&Transform, &mut Velocity), (With<Boss>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    const SPEED: f32 = 3.0;
    for (boss_transform, mut boss_velocity) in bosses.iter_mut() {
        // Get vector representing direction from enemy to player
        let mut direction_vec =
            player_transform.single_mut().translation - boss_transform.translation;
        direction_vec = direction_vec.normalize();

        // Calculate distance of vectors, so enemy chases player only until it's near him
        let vec2_player = Vec2::new(
            player_transform.single_mut().translation[0],
            player_transform.single_mut().translation[2],
        );
        let vec2_enemy = Vec2::new(
            boss_transform.translation[0],
            boss_transform.translation[2],
        );
        if vec2_player.distance(vec2_enemy) > 2.0 {
            boss_velocity.linvel[0] = direction_vec[0] * SPEED;
            boss_velocity.linvel[2] = direction_vec[2] * SPEED;
        }
    }
}

fn boss_melee_attack(
    mut enemies: Query<(&Transform, &mut Boss), With<Boss>>,
    mut player: Query<(Entity, &mut Health, &Transform), With<Player>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
    mut timer: ResMut<BossAttackTimer>,
    mut floating_text_event_writer: EventWriter<FloatingTextEvent>,
) {
    for (boss_transform, mut boss) in enemies.iter_mut() {
        let shape = Collider::ball(2.0);
        let shape_pos = boss_transform.translation;
        let shape_rot = boss_transform.rotation;
        let filter = QueryFilter::default();

        let mut player_is_hit = false;
        rapier_context.intersections_with_shape(shape_pos, shape_rot, &shape, filter, |entity| {
            if entity == player.single().0 {
                player_is_hit = true;
            }
            true
        });

        if player_is_hit {
            match boss.boss_state {
                BossState::Attacking => {
                    // Attack player
                    player.single_mut().1.0 -= 1;

                    // Create floating text
                    floating_text_event_writer.send(FloatingTextEvent {
                        translation: player.single_mut().2.translation,
                        text: "-1".into(),
                        color: Color::rgb(0.7, 0.0, 0.0),
                    });

                    boss.boss_state = BossState::Cooldown;
                }
                _ => {
                    // Boss attack cooldown
                    if !timer.0.tick(time.delta()).finished() {
                        continue;
                    }
                    boss.boss_state = BossState::Attacking;
                }
            }
        }
    }
}

fn boss_shoot_attack(
    mut player: Query<(&Transform, &mut Health), (With<Player>, Without<Boss>)>,
    mut bosses: Query<
        (Entity, &Transform, &mut Boss),
        (With<Boss>, Without<Player>),
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<BossAttackTimer>,
) {
    for (boss_entity, boss_transform, mut boss) in bosses.iter_mut() {

        let direction =
            (player.single_mut().0.translation - boss_transform.translation).normalize();

        match boss.boss_state {
            BossState::Attacking => {
                match boss.boss_type {
                    BossType::Boss1 => {
                        boss1_attack(boss_entity, *boss_transform, direction, &mut meshes, &mut materials, &mut commands);
                    }
                    BossType::Boss2 => {}
                }

                boss.boss_state = BossState::Cooldown;
            }
            _ => {
                // Enemy attack cooldown
                if !timer.0.tick(time.delta()).finished() {
                    continue;
                }
                boss.boss_state = BossState::Attacking;
            }
        }
    }
}

fn boss1_attack(
    boss_entity: Entity,
    boss_transform: Transform,
    direction: Vec3,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>, 
    mut commands: &mut Commands,
) {
    let translation = boss_transform.translation - Vec3::new(0.0, 1.0, 0.0);
    let mut rng = rand::thread_rng();
    match rng.gen_range(1..=3) {
        1 => {
            spawn_bullet(translation, direction, boss_entity, &mut meshes, &mut materials, &mut commands)
        },
        2 => {
            spawn_bullet(translation, Quat::from_rotation_y(-0.3) * direction, boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, direction, boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Quat::from_rotation_y(0.3) * direction, boss_entity, &mut meshes, &mut materials, &mut commands)
        },
        3 => {
            spawn_bullet(translation, Vec3::new(1.0, 0.0, 0.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(0.0, 0.0, 1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(0.0, 0.0, -1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(-1.0, 0.0, 0.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(1.0, 0.0, 1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(-1.0, 0.0, 1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(1.0, 0.0, -1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
            spawn_bullet(translation, Vec3::new(-1.0, 0.0, -1.0), boss_entity, &mut meshes, &mut materials, &mut commands);
        },
        _ => unreachable!()
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
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(origin),
            ..default()
        })
        .insert(BossBullet{
            shooter,
            direction,
            start_position: origin
        })
        .insert(RigidBody::Dynamic)
        .insert(Velocity::zero());
}