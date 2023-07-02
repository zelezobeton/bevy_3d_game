use std::f32::consts::PI;
use rand::Rng;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::{GameState, Health};

pub enum BossState {
    Attacking,
    Cooldown,
}

#[derive(PartialEq)]
pub enum BossType {
    Boss1
}

#[derive(Component)]
pub struct Boss {
    boss_type: BossType,
    boss_state: BossState
}

#[derive(Resource)]
struct BossSpawnTimer(Timer);

pub struct BossesPlugin;
impl Plugin for BossesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BossSpawnTimer(Timer::from_seconds(
            20.0,
            TimerMode::Repeating,
        )))
        .add_systems(
            (
                spawn_bosses,
                rotate_boss
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
    if timer.0.tick(time.delta()).elapsed_secs() != 0.0 && !timer.0.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    match rng.gen_range(1..=1) {
        1 => {
            spawn_boss(BossType::Boss1, "models/characterAlien.glb#Scene0", &mut commands, &asset_server)
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
        .insert(Health(5))
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