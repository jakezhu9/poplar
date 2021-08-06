#![feature(bool_to_option, type_ascription)]

mod cargo;
mod flags;
mod image;
mod qemu;

use eyre::Result;
use qemu::RunQemuX64;
use std::{
    env,
    path::{Path, PathBuf},
};
use xshell::pushd;

fn main() -> Result<()> {
    color_eyre::install()?;
    let _root = pushd(project_root())?;

    let flags = flags::Task::from_env()?;
    match flags.subcommand {
        flags::TaskCmd::Help(_) => {
            println!("{}", flags::Task::HELP);
            Ok(())
        }

        flags::TaskCmd::Dist(dist) => make_dist(dist.release),

        flags::TaskCmd::Qemu(qemu) => {
            make_dist(qemu.release)?;
            RunQemuX64::new(PathBuf::from("pebble.img"))
                .open_display(qemu.display)
                .debug_int_firehose(qemu.debug_int_firehose)
                .debug_mmu_firehose(qemu.debug_mmu_firehose)
                .debug_cpu_firehose(qemu.debug_cpu_firehose)
                .run()
        }
    }
}

fn make_dist(release: bool) -> Result<()> {
    use cargo::{RunCargo, Target};
    use image::MakeGptImage;

    let efiloader_path = RunCargo::new("efiloader.efi".to_string(), PathBuf::from("kernel/efiloader/"))
        .workspace(PathBuf::from("kernel/"))
        .target(Target::Triple("x86_64-unknown-uefi".to_string()))
        .release(release)
        .std_components(vec!["core".to_string()])
        .std_features(vec!["compiler-builtins-mem".to_string()])
        .run()?;

    let kernel_path = RunCargo::new("kernel_x86_64".to_string(), PathBuf::from("kernel/kernel_x86_64/"))
        .workspace(PathBuf::from("kernel/"))
        .target(Target::Custom {
            triple: "x86_64-kernel".to_string(),
            spec: PathBuf::from("kernel/kernel_x86_64/x86_64-kernel.json"),
        })
        .release(release)
        .std_components(vec!["core".to_string(), "alloc".to_string()])
        .run()?;

    MakeGptImage::new(PathBuf::from("pebble.img"), 30 * 1024 * 1024, 20 * 1024 * 1024)
        .add_efi_file("efi/boot/bootx64.efi".to_string(), efiloader_path)
        .add_efi_file("kernel.elf".to_string(), kernel_path)
        .build()?;

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned()))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}
