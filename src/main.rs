extern crate cgmath;
extern crate getopts;
extern crate gif;
extern crate ordered_float;
extern crate sdl2;

mod asyncrenderer;
mod color;
mod fastmath;
mod file;
mod formulas;
mod genetics;
mod interactive;
mod renderer;
mod settings;

use color::colormapper::{NUM_COLOR_GENES, CONTROL_POINT_GENE_SIZE};
use formulas::{NUM_FORMULA_GENES, FORMULA_GENE_SIZE};
use getopts::{Matches, Options};
use genetics::{Chromosome, Genome, Population};
use settings::{GeneticSettings, OutputMode, OutputSettings, PlasmaSettings, RenderingSettings};
use std::cmp::max;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::exit;

const STARTING_POPULATION_SIZE: usize = 8;
const MAX_POPULATION_SIZE: usize = 32;

macro_rules! errorln {
    ($x:expr, $($y:tt)*) => { writeln!(&mut std::io::stderr(), $x, $($y)*).unwrap() };
}

fn main() {
    let opts = create_options();
    let matches = match opts.parse(env::args()) {
        Ok(m) => m,
        Err(e) => exit_with_error(&format!("{}", e), true)
    };
    if matches.opt_present("help") {
        exit_with_help();
    }
    let params = match build_plasma_settings(matches) {
        Ok(params) => params,
        Err(message) => exit_with_error(&message, true)
    };

    match params.output.mode.clone() {
        OutputMode::File{ref path} => {
            File::create(path).and_then(|mut file| {
                let gif_bytes = file::generate_gif_bytes(params);
                file.write_all(&gif_bytes[..])
            }).unwrap_or_else(|e|
                exit_with_error(&format!("Couldn't write to {}: {}", &path, e), false)
            );
        },
        OutputMode::Interactive => interactive::run_interactive(params)
    };
}

fn get_program_name() -> String {
    env::args().nth(0).unwrap_or("plasma".to_string())
}

fn exit_with_error(message: &str, suggest_help: bool) -> ! {
    let program_name = get_program_name();
    errorln!("{program}: {message}", program = program_name, message = message);
    if suggest_help {
        errorln!("Run '{program} --help' for more information.", program = program_name);
    }
    exit(1)
}

fn exit_with_help() -> ! {
    let program_name = get_program_name();
    let header = format!(
        "\
            Usage: {program} [OPTION]... [GENOME]...\n\
            GENOME is a Base64 string that represents a plasma's pattern and color.\n\
            More than one genome can be specified.\
        ",
        program = program_name
    );
    // TODO: Add instructions for interactive mode
    println!("{}", create_options().usage(&header));
    exit(0)
}

fn create_options() -> Options {
    let mut opts = Options::new();
    opts.optflag("d", "dithering", "Force dithering");
    opts.optopt("p", "palette", "Render using a color palette of a given size", "N");
    opts.optopt("f", "fps", "Frames per second", "N");
    opts.optopt("l", "loop-duration", "Seconds until the animation loops", "N");
    opts.optopt("i", "input", "Read genomes from file, one genome per line", "FILE");
    opts.optopt("o", "output", "Output to a file (GIF) instead of to a window", "FILE");
    opts.optopt("w", "width", "Width, in pixels", "X");
    opts.optopt("h", "height", "Height, in pixels", "Y");
    opts.optflag("", "help", "Show this help text");
    opts
}

fn build_plasma_settings(matches: Matches) -> Result<PlasmaSettings, String> {
    // Read genomes from free arguments
    let genome_strings = &matches.free[1..];
    let mut genomes = vec![];
    for genome_string in genome_strings {
        match Genome::from_base64(genome_string) {
            Ok(g) => genomes.push(g),
            Err(..) => return Err(format!("Couldn't parse {}", genome_string))
        };
    }

    // Read additional genomes from file
    if let Some(filename) = matches.opt_str("i") {
        File::open(&filename).and_then(|file| {
            for line_result in BufReader::new(file).lines() {
                let line = line_result?;
                if let Ok(g) = Genome::from_base64(line.trim()) {
                    genomes.push(g);
                }
            }
            Ok(())
        }).unwrap_or_else(
            |e| exit_with_error(&format!("Couldn't read from {}: {}", &filename, e), false)
        );
    }

    // Set up genetic settings
    if genomes.len() == 0 {
        for _ in 0..STARTING_POPULATION_SIZE {
            genomes.push(Genome {
                pattern: Chromosome::rand(NUM_FORMULA_GENES, FORMULA_GENE_SIZE),
                color: Chromosome::rand(NUM_COLOR_GENES, CONTROL_POINT_GENE_SIZE)
            });
        }
    }
    let starting_genome = genomes[0].clone();
    let mut population = Population::new(max(MAX_POPULATION_SIZE, genomes.len()));
    for genome in genomes {
        population.add(genome);
    }
    let genetic_settings = GeneticSettings {
        genome: starting_genome,
        population: population
    };

    // Set up output settings
    let output_mode = if let Some(path) = matches.opt_str("o") {
        OutputMode::File { path }
    } else {
        OutputMode::Interactive
    };
    let output_settings = OutputSettings {
        mode: output_mode
    };

    // Set up rendering settings
    let mut rendering_settings = match output_settings.mode {
        OutputMode::Interactive => RenderingSettings {
            dithering: false,
            frames_per_second: 16.0,
            loop_duration: 60.0,
            palette_size: None,
            width: 640,
            height: 480
        },
        OutputMode::File{..} => RenderingSettings {
            dithering: true,
            frames_per_second: 10.0,
            loop_duration: 60.0,
            palette_size: Some(64),
            width: 320,
            height: 240
        }
    };
    if matches.opt_present("d") {
        rendering_settings.dithering = true;
        if rendering_settings.palette_size.is_none() {
            rendering_settings.palette_size = Some(255);
        }
    }
    if let Some(fps_str) = matches.opt_str("f") {
        rendering_settings.frames_per_second = match fps_str.parse() {
            Ok(f) if f > 0.0 => f,
            _ => return Err(format!("Not a positive number: {}", fps_str))
        };
    }
    if let Some(loop_duration_str) = matches.opt_str("l") {
        rendering_settings.loop_duration = match loop_duration_str.parse() {
            Ok(n) if n > 0.0 => n,
            _ => return Err(format!("Not a positive number: {}", loop_duration_str))
        };
    }
    if let Some(palette_size_str) = matches.opt_str("p") {
        // TODO: Add support for 256 colors
        rendering_settings.palette_size = match palette_size_str.parse() {
            Ok(n) if 2 <= n && n <= 255 => Some(n),
            _ => return Err(format!("Not an integer from 2 to 255: {}", palette_size_str))
        };
    }
    if let (Some(width_str), Some(height_str)) = (matches.opt_str("w"), matches.opt_str("h")) {
        rendering_settings.width = match width_str.parse() {
            Ok(w) if w > 0 => w,
            _ => return Err(format!("Not a positive integer: {}", width_str))
        };
        rendering_settings.height = match height_str.parse() {
            Ok(h) if h > 0 => h,
            _ => return Err(format!("Not a positive integer: {}", height_str))
        };
    } else if matches.opt_present("w") != matches.opt_present("h") {
        return Err("Width and height must both be specified".to_string());
    }

    Ok(PlasmaSettings {
        genetics: genetic_settings,
        rendering: rendering_settings,
        output: output_settings
    })
}
