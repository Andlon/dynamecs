use eyre::eyre;
use eyre::Context;
use log::info;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;
use std::{fmt, fs};

use dynamecs::components::{get_step_index, try_get_settings};
use dynamecs::{ObserverSystem, Universe};

/// Tries to deserialize a [`dynamecs::Universe`] from the specified file path.
///
/// The file format is inferred from the file extension.
pub fn restore_checkpoint_file<P: AsRef<Path>>(checkpoint_path: P) -> eyre::Result<Universe> {
    let checkpoint_path = checkpoint_path.as_ref();
    // Extract file extension
    let extension = checkpoint_path
        .extension()
        .and_then(OsStr::to_str)
        .ok_or_else(|| {
            eyre!(
                "failed to determine extension of checkpoint file \"{}\"",
                checkpoint_path.display()
            )
        })?;

    // Call the right deserializer depending on the file extension
    match extension.to_lowercase().as_str() {
        "bin" => restore_compressed_binary_checkpoint_file(checkpoint_path),
        _ => {
            return Err(eyre!(
                "Unsupported file extension \"{}\" of checkpoint file \"{}\"",
                extension,
                checkpoint_path.display()
            ));
        }
    }
    .wrap_err_with(|| {
        format!(
            "failed to restore checkpoint from file \"{}\"",
            checkpoint_path.display()
        )
    })
}

fn restore_compressed_binary_checkpoint_file<P: AsRef<Path>>(checkpoint_path: P) -> eyre::Result<Universe> {
    let checkpoint_path = checkpoint_path.as_ref();
    let checkpoint_file = fs::OpenOptions::new()
        .read(true)
        .create(false)
        .open(checkpoint_path)
        .wrap_err("failed to open checkpoint file for reading")?;

    let uncompressed_file_stream = snap::read::FrameDecoder::new(checkpoint_file);
    bincode::deserialize_from(uncompressed_file_stream).wrap_err("error during deserialization of checkpoint file")
}

/// Returns a checkpointing system that serializes the [`dynamecs::Universe`] at every timestep using `bincode` and compressed with `snap`.
pub fn compressed_binary_checkpointing_system() -> impl ObserverSystem {
    CheckpointingSystem::new(|file, universe| {
        let compressed_file_stream = snap::write::FrameEncoder::new(file);
        bincode::serialize_into(compressed_file_stream, universe)?;
        Ok(())
    })
}

/// Generic checkpointing system independent from the serialization file format.
struct CheckpointingSystem<SerializeFn> {
    serializer: SerializeFn,
}

impl<SerializeFn> Debug for CheckpointingSystem<SerializeFn> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CheckpointingSystem")
    }
}

impl<SerializeFn> CheckpointingSystem<SerializeFn>
where
    SerializeFn: FnMut(fs::File, &Universe) -> eyre::Result<()>,
{
    /// Constructs a checkpointing system from the given `FnMut(fs::File, &Universe) -> eyre::Result<()>` serialization closure.
    fn new(serializer: SerializeFn) -> Self {
        Self { serializer }
    }
}

impl<SerializeFn> ObserverSystem for CheckpointingSystem<SerializeFn>
where
    SerializeFn: FnMut(fs::File, &Universe) -> eyre::Result<()>,
{
    fn name(&self) -> String {
        "CheckpointingSystem".to_string()
    }

    fn run(&mut self, universe: &Universe) -> eyre::Result<()> {
        // Ensure that all components in the universe are registered
        let unregistered_components = universe.unregistered_components();
        if !unregistered_components.is_empty() {
            return Err(eyre!(
                "the following components are not registered: {:?}",
                &unregistered_components
            ));
        }

        let settings = try_get_settings(universe)?;
        let checkpoint_path = settings.output_folder.join("checkpoints");
        // Ensure that the checkpoint output folder exists
        fs::create_dir_all(&checkpoint_path).wrap_err_with(|| {
            format!(
                "failed to create output directory for checkpoints \"{}\"",
                checkpoint_path.display()
            )
        })?;

        let step_index = get_step_index(universe).0;

        let checkpoint_file_name = format!("checkpoint_{}.bin", step_index);
        let checkpoint_file_path = checkpoint_path.join(checkpoint_file_name);

        // Open checkpoint file for writing
        let checkpoint_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            // TODO: To append or to truncate? Return error when file exists?
            .truncate(true)
            .append(false)
            .open(&checkpoint_file_path)
            .wrap_err_with(|| {
                format!(
                    "unable to open checkpoint file '{}' for writing",
                    checkpoint_file_path.display(),
                )
            })?;

        // Run the serializer
        info!("Writing checkpoint to file \"{}\"...", checkpoint_file_path.display());
        (self.serializer)(checkpoint_file, universe).wrap_err("error during serialization for checkpoint")?;

        Ok(())
    }
}
