use eyre::eyre;
use std::fmt::Debug;
use std::path::PathBuf;
use std::{fmt, fs};

use dynamecs::components::{try_get_settings, DynamecsAppSettings};
use dynamecs::{ObserverSystem, Universe};

fn compressed_binary_checkpointing_system(settings: &DynamecsAppSettings) -> impl ObserverSystem {
    CheckpointingSystem::new(settings, |file, universe| {
        let compressed_file_stream = snap::write::FrameEncoder::new(file);
        bincode::serialize_into(compressed_file_stream, universe)?;
        Ok(())
    })
}

struct CheckpointingSystem<SerializeFn> {
    checkpoint_path: PathBuf,
    checkpoint_index: usize,
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
    fn new(settings: &DynamecsAppSettings, serializer: SerializeFn) -> Self {
        let checkpoint_path = settings.output_folder.join("checkpoints");

        Self {
            checkpoint_path,
            checkpoint_index: 0,
            serializer,
        }
    }
}

impl<SerializeFn> ObserverSystem for CheckpointingSystem<SerializeFn>
where
    SerializeFn: FnMut(fs::File, &Universe) -> eyre::Result<()>,
{
    fn run(&mut self, universe: &Universe) -> eyre::Result<()> {
        let settings = try_get_settings(universe)?;
        let checkpoint_file_name = format!("{}_checkpoint_{}", &settings.scenario_name, self.checkpoint_index);
        let checkpoint_file_path = self.checkpoint_path.join(checkpoint_file_name);

        // Open checkpoint file for writing
        let checkpoint_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            // TODO: To append or to truncate? Return error when file exists?
            .truncate(true)
            .append(false)
            .open(&checkpoint_file_path)
            .map_err(|e| {
                eyre!(
                    "unable to open checkpoint file '{}' for writing ({:?})",
                    checkpoint_file_path.display(),
                    e
                )
            })?;

        // Run the serializer
        (self.serializer)(checkpoint_file, universe)?;

        self.checkpoint_index += 1;

        Ok(())
    }
}
