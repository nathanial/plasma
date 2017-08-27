use genetics::Genome;
use renderer::{Image, PlasmaRenderer};
use settings::RenderingSettings;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, RecvError};
use futures::{Future, BoxFuture};
use futures_cpupool::{CpuPool, CpuFuture};

pub struct AsyncRenderer {
    last_request_id: u32,
    genome: Option<Genome>,
    genome_set: bool,
    settings: RenderingSettings,
    pool: CpuPool
}

struct Request {
    request_id: u32,
    genome: Option<Genome>,
    width: usize,
    height: usize,
    time: f32
}

struct Response {
    image: Image,
    request_id: u32
}

impl AsyncRenderer {
    pub fn new(settings: &RenderingSettings) -> AsyncRenderer {
        let pool = CpuPool::new_num_cpus();
        let settings_clone = settings.clone();
        AsyncRenderer {
            last_request_id: 0,
            genome: None,
            genome_set: false,
            settings: settings_clone,
            pool: pool
        }
    }

    fn next_request_id(&mut self) -> u32 {
        self.last_request_id = self.last_request_id.wrapping_add(1);
        self.last_request_id
    }

    pub fn set_genome(&mut self, genome: &Genome) {
        self.genome = Some(genome.clone());
        println!("WE SET THE GENEOME {:?}", self.genome);
        self.genome_set = true;
        self.next_request_id(); // Increment request ID to invalidate previous requests
    }

    pub fn render(&mut self, width: usize, height: usize, time: f32) -> CpuFuture<Image, u32> {
        assert!(self.genome_set, "Must call set_genome() before calling render()");
        let genome = self.genome.clone().unwrap();
        let settings = self.settings.clone();
        let mut renderer = PlasmaRenderer::new(&genome, &settings);
        return self.pool.spawn_fn(move || {
            let mut image = Image::new(width, height);
            renderer.render(&mut image, time);
            return Ok(image)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color::colormapper::{CONTROL_POINT_GENE_SIZE, NUM_COLOR_GENES};
    use formulas::{FORMULA_GENE_SIZE, NUM_FORMULA_GENES};
    use genetics::{Chromosome, Genome};
    use renderer::{Image, PlasmaRenderer};
    use settings::RenderingSettings;
    use std::thread::sleep;
    use std::time::Duration;

    /*
     *  Helper functions
     */

    fn dummy_settings() -> RenderingSettings {
        RenderingSettings {
            dithering: false,
            frames_per_second: 16.0,
            loop_duration: 60.0,
            palette_size: None,
            width: 32,
            height: 32
        }
    }

    fn rand_genome() -> Genome {
        Genome {
            pattern: Chromosome::rand(NUM_FORMULA_GENES, FORMULA_GENE_SIZE),
            color: Chromosome::rand(NUM_COLOR_GENES, CONTROL_POINT_GENE_SIZE)
        }
    }

    /*
     *  Tests
     */

    #[test]
    fn test_asyncrenderer_singlerender() {
        // Make a request
        let genome = rand_genome();
        let mut ar = AsyncRenderer::new(&dummy_settings());
        ar.set_genome(&genome);
        let image1 = ar.render(32, 32, 0.0).get();

        // Compare image with regular Renderer
        let mut r = PlasmaRenderer::new(&genome, &dummy_settings());
        let mut image2 = Image::new(32, 32);
        r.render(&mut image2, 0.0);
        assert_eq!(image1.pixel_data, image2.pixel_data);
    }

    #[test]
    fn test_asyncrenderer_cancellation() {
        // Warm up the AsyncRenderer by making a small request and waiting for it to finish
        let mut ar = AsyncRenderer::new(&dummy_settings());
        ar.set_genome(&rand_genome());
        ar.render(2, 2, 0.0);
        wait_for_image(&mut ar);

        // Start of actual test: Render image A, but don't retrieve the result
        ar.render(3, 3, 0.25);
        sleep(Duration::from_millis(10)); // Wait for render to complete

        // Render image B
        ar.render(5, 5, 0.5);

        // Assert that image A is no longer available
        assert!(ar.get_image().is_none());

        // Assert that we eventually get image B (and not A)
        let image = wait_for_image(&mut ar);
        assert_eq!(image.width, 5);
    }
}
