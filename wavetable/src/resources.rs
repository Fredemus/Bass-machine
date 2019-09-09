use std::{io, iter::once, path::PathBuf};

use dirs;
use either::{Left, Right};
use hound::{self, WavReader};
use lazy_static::lazy_static;

#[derive(Clone, Debug)]
pub enum Table {
    BuiltIn(Vec<f32>),
    WavFile(PathBuf),
}

impl Table {
    pub fn load(self) -> Result<Vec<f32>, hound::Error> {
        match self {
            Table::BuiltIn(samples) => Ok(samples),
            Table::WavFile(path) => {
                let mut reader = WavReader::open(path)?;
                reader.samples().collect()
            }
        }
    }
}

/// Iterates over all preset table directories.
fn preset_table_dirs() -> impl Iterator<Item=PathBuf> {
    PRESET_DIRS.iter()
        .map(|presets| presets.join("Tables"))
        .filter(|path| path.is_dir())
}

fn flatten_nested_results<T, E, II, IO>(iter_outer: IO) -> impl Iterator<Item=Result<T, E>>
where
    II: Iterator<Item = Result<T, E>>,
    IO: Iterator<Item = Result<II, E>>,
{
    iter_outer.flat_map(|iter_inner_result| match iter_inner_result {
        Ok(iter_inner) => Left(iter_inner),
        Err(err) => Right(once(Err(err))),
    })
}

/// Iterates over all wavetable paths in preset table directories.
fn preset_table_dir_tables() -> impl Iterator<Item=io::Result<PathBuf>> {
    flatten_nested_results(
        preset_table_dirs()
            .map(|preset_table_dir| preset_table_dir.read_dir())
    ).map(|result| result.map(|dir_entry| dir_entry.path()))
}

/// Gets all preset and built-in wavetables.
pub fn tables() -> io::Result<Vec<Table>> {
    BUILTINS.iter()
        .map(|builtin| Ok(builtin.clone()))
        .chain(
            preset_table_dir_tables()
                .map(|path_buf| path_buf.map(|path_buf| Table::WavFile(path_buf)))
        )
        .collect()
}

lazy_static! {
    static ref BUILTINS: Vec<Table> = {
        macro_rules! include_wav {
            ( $path:expr ) => {{
                let bytes = include_bytes!($path);
                let reader = WavReader::new(&bytes[..]).unwrap();
                let samples = reader.into_samples().collect::<Result<_, _>>().unwrap();
                Table::BuiltIn(samples)
            }}
        }

        vec![
            include_wav!("../resources/tables/Basic Shapes.wav"),
        ]
    };

    static ref PRESET_DIRS: Vec<PathBuf> = {
        #[cfg(windows)] {
            vec![
                dirs::document_dir().unwrap()
                    .join("Graintable/Graintable Presets"),
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                PathBuf::from("/Library/Audio/Presets/Graintable/Graintable Presets"),
                dirs::home_dir().unwrap()
                    .join("Library/Audio/Presets/Graintable/Graintable Presets"),
            ]
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            vec![
                dirs::data_dir().unwrap()
                    .join("Graintable/Graintable Presets"),
            ]
        }
    };
}

#[test]
#[ignore]
fn print_tables() {
    let tables = tables().unwrap();
    println!("{:#?}", tables);
}
