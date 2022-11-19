use std::f32::consts::PI;

use bevy::{
    math::vec3,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
pub struct Shape;

const X_EXTENT: f32 = 14.;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let shapes = [
        meshes.add(shape::Cube::default().into()),
        meshes.add(shape::Box::default().into()),
        meshes.add(shape::Capsule::default().into()),
        meshes.add(shape::Torus::default().into()),
        meshes.add(shape::Icosphere::default().try_into().unwrap()),
        meshes.add(shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane { size: 50. }.into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

const FILE_NAME: &str = "H:/rustcache/debug/ridiculous_bevy_hot_reloading.dll";

macro_rules! make_hot {
    ($arg_names:tt $arg_types:tt pub fn $fn_name:ident $prams:tt $bl:block) => {
        ::paste::paste! {
            #[no_mangle]
            fn [<ridiculous_bevy_hot_ $fn_name >] $prams $bl

            pub fn $fn_name $prams {
                unsafe {
                    let s = concat!("ridiculous_bevy_hot_", stringify!($fn_name));
                    if let Ok(lib) = libloading::Library::new(FILE_NAME) {
                        let func: libloading::Symbol<
                            unsafe extern "C" fn $arg_types ,
                        > = lib.get(s.as_bytes()).unwrap();
                        func $arg_names;
                    } else {
                        [<ridiculous_bevy_hot_ $fn_name >] $arg_names;
                    }
                }

            }
        }
    };
}

make_hot!(
    (query, time)
    (Query<&mut Transform, With<Shape>>, Res<Time>)
    pub fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
        for mut transform in &mut query {
            transform.rotate_x(time.delta_seconds() * 1.0);
        }
    }
);

make_hot!(
    (query, time)
    (Query<&mut Transform, With<Shape>>, Res<Time>)
    pub fn rotate2(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
        for mut transform in &mut query {
            let rot = Quat::from_rotation_y(0.1 * time.delta_seconds());
            transform.translate_around(vec3(0.0,0.0,0.0), rot);
        }
    }
);

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    )
}
