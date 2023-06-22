/*
TODO:
- Add boss with few special attacks
- Add text information when attacks happens (floating text)
- Create custom meshes for enemies? (choose theme)

LONGTERM:
- Add levels with different layout, platforms etc.

DONE:
- Add different kinds of enemies
  - One with shotgun shooting 3 balls
  - One that shoots 8 balls around itself
- Spawn creatures that interact with character, chase him, hurt him, etc.
- Make custom character in Blender and animate player attacking
- Refine player movement
- Make enemies spawn periodically
- Make enemy attack player
- Make player attack enemy
- Add different type of enemy, that stands still and shoots bullets
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

use rand::Rng;

use bevy::render::mesh::shape as render_shape;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_rapier3d::prelude::*;

mod player;
mod enemies;
mod bosses;
use player::Player;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    Playing,
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Cursor;

#[derive(Component)]
struct BonusComponent;

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct HealthText;

#[derive(Component)]
pub struct FloatingText {
    pub offset: f32,
    pub time_to_live: f32,
}

pub struct FloatingTextEvent {
    pub translation: Vec3,
    pub text: String,
    pub color: Color,
}

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
    player: Option<Entity>,
}

#[derive(Resource)]
struct BonusSpawnTimer(Timer);

const DEFAULT_PLAYER_POS: [f32; 3] = [0.0, 1.0, 0.0];
const DEFAULT_CAMERA_POS: [f32; 3] = [-7.0, 10.0, 0.0];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(player::PlayerPlugin)
        .add_plugin(enemies::EnemiesPlugin)
        .add_plugin(bosses::BossesPlugin)
        .init_resource::<Game>()
        .insert_resource(BonusSpawnTimer(Timer::from_seconds(
            5.0,
            TimerMode::Repeating,
        )))
        .add_state::<GameState>()
        .add_systems((
            setup_camera.on_startup(),
            setup_light.on_startup(),
            spawn_level.on_startup(),
            setup.in_schedule(OnEnter(GameState::Playing)),
        ))
        .add_systems(
            (
                move_cursor,
                move_camera,
                spawn_bonus,
                show_health,
                spawn_bonus,
                get_bonus,
                create_floating_text
            )
            .in_set(OnUpdate(GameState::Playing)),
        )
        .add_event::<FloatingTextEvent>()
        .add_system(bevy::window::close_on_esc)
        .run();
}

fn create_floating_text(
    mut commands: Commands,
    camera: Query<(&Camera, &mut GlobalTransform), (With<MainCamera>, Without<Player>)>,
    mut attack_text: Query<(Entity, &mut FloatingText)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut floating_text_event_reader: EventReader<FloatingTextEvent>,
) {
    for event in floating_text_event_reader.iter() {
        for (camera, global_transform) in camera.iter() {
            match camera.world_to_viewport(global_transform, event.translation) {
                Some(coords) => {
                    commands.spawn(
                        TextBundle::from_section(
                            event.text.clone(),
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: event.color,
                            },
                        )
                        .with_style(Style {
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                bottom: Val::Px(coords.y),
                                left: Val::Px(coords.x),
                                ..default()
                            },
                            ..default()
                        }),
                    )
                    .insert(FloatingText{offset: 0.0, time_to_live: 1.0});
                }
                None => {}
            }
        }
    }

    // Calculate when floating text should be hidden 
    for (entity, mut attack_text_struct) in attack_text.iter_mut() {
        attack_text_struct.offset += time.delta_seconds();

        if attack_text_struct.offset > attack_text_struct.time_to_live {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn setup(asset_server: Res<AssetServer>, mut game: ResMut<Game>, mut commands: Commands) {
    // load the scene for the bonus
    game.bonus.handle = asset_server.load("models/pumpkin.glb#Scene0");

    // scoreboard
    commands.spawn(
        TextBundle::from_section(
            "Score:",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 40.0,
                color: Color::rgb(0.7, 0.0, 0.0),
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
    ).insert(HealthText);

    // Setup cursor
    commands
        .spawn(SceneBundle {
            transform: Transform {
                translation: Vec3::ZERO,
                scale: Vec3::new(2.0, 2.0, 2.0),
                ..default()
            },
            // scene: game.bonus.handle.clone(),
            ..default()
        })
        .insert(Cursor);
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
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 7000.0,
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

fn move_camera(
    mut camera_transform: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    mut player_transform: Query<&Transform, (With<Player>, Without<MainCamera>)>,
) {
    let player_pos = player_transform.single_mut().translation;

    let camera_distance: Vec3 = Vec3::from(DEFAULT_CAMERA_POS) - Vec3::from(DEFAULT_PLAYER_POS);
    let new_camera_pos = player_pos + camera_distance;

    // Interpolated camera movement
    camera_transform.single_mut().translation = camera_transform
        .single_mut()
        .translation
        .lerp(new_camera_pos, 0.2);
}

fn move_cursor(
    rapier_context: Res<RapierContext>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), (With<MainCamera>, Without<Player>)>,
    mut cursor_transform: Query<&mut Transform, With<Cursor>>,
) {
    let (camera, camera_transform) = q_camera.single();
    let Ok(primary) = primary_query.get_single() else {
        return;
    };
    if let Some(screen_pos) = primary.cursor_position() {
        let world_ray = camera
            .viewport_to_world(camera_transform, screen_pos)
            .unwrap();

        let ray_pos = world_ray.origin;
        let ray_dir = world_ray.direction;
        let max_toi = 100.0;
        let solid = true;
        let filter = QueryFilter { ..default() };
        if let Some((_entity, intersection)) =
            rapier_context.cast_ray_and_get_normal(ray_pos, ray_dir, max_toi, solid, filter)
        {
            let hit_point = intersection.point;
            cursor_transform.single_mut().translation = hit_point;
        }
    }
}

fn get_bonus(
    mut player: Query<(Entity, &mut Health), With<Player>>,
    bonus: Query<(Entity, &Transform), With<BonusComponent>>,
    mut game: ResMut<Game>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
) {
    for (bonus_entity, bonus_transform) in bonus.iter() {
        let shape = Collider::ball(0.5);
        let shape_pos = bonus_transform.translation;
        let shape_rot = bonus_transform.rotation;
        let filter = QueryFilter::default();

        rapier_context.intersections_with_shape(shape_pos, shape_rot, &shape, filter, |entity| {
            if entity == player.single().0 {
                commands.entity(bonus_entity).despawn_recursive();
                game.bonus.entity = None;

                // Add player health
                player.single_mut().1 .0 += 1;
            }
            true
        });
    }
}

fn spawn_level(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let ground_size = 20.0;
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
        .insert(Collider::cuboid(
            ground_size / 2.0,
            ground_height / 2.0,
            ground_size / 2.0,
        ))
        .insert(Transform::from_xyz(0.0, -ground_height, 0.0))
        .insert(GlobalTransform::default());
}

// despawn the bonus if there is one, then spawn a new one at a random location
fn spawn_bonus(
    time: Res<Time>,
    mut timer: ResMut<BonusSpawnTimer>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut player_transform: Query<&Transform, With<Player>>,
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
        let player_pos = Vec2::new(
            player_transform.single_mut().translation[0],
            player_transform.single_mut().translation[2],
        );
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
                    scale: Vec3::new(2.0, 2.0, 2.0),
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

// Update the health displayed during the game
fn show_health(
    mut text_query: Query<&mut Text, With<HealthText>>,
    mut health_query: Query<&Health, With<Player>>,
) {
    let mut text = text_query.single_mut();
    let health = health_query.single_mut();
    text.sections[0].value = format!("Health: {}", health.0);
}
