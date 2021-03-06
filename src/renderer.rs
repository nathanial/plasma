use color::Color;
use color::colormapper::ColorMapper;
use fastmath::FastMath;
use formulas::PlasmaFormulas;
use genetics::Genome;
use settings::RenderingSettings;
use std::f32;

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixel_data: Vec<u8>
}

pub struct PlasmaRenderer {
    dithering: bool,
    color_mapper: ColorMapper,
    formulas: PlasmaFormulas
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            width: width,
            height: height,
            pixel_data: vec![0; width*height*3]
        }
    }

    pub fn plot(&mut self, x: usize, y: usize, color: Color) {
        let offset = (x + y*self.width)*3;
        self.pixel_data[offset] = color.r;
        self.pixel_data[offset + 1] = color.g;
        self.pixel_data[offset + 2] = color.b;
    }
}

impl PlasmaRenderer {
    pub fn new(genome: &Genome, settings: &RenderingSettings) -> PlasmaRenderer {
        let color_mapper = ColorMapper::new(&genome.color, &settings);
        let formulas = PlasmaFormulas::from_chromosome(&genome.pattern);
        PlasmaRenderer {
            color_mapper: color_mapper,
            dithering: settings.dithering,
            formulas: formulas
        }
    }

    pub fn render(&mut self, image: &mut Image, time: f32) {
        // Scale screen coordinates so the smaller dimension ranges from -1.0 to 1.0
        let scale_mul = 2.0/((image.width as f32).min(image.height as f32));
        let scale_x_offset = -(image.width as f32)/2.0*scale_mul;
        let scale_y_offset = -(image.height as f32)/2.0*scale_mul;
        let adj_time = time.wrap();
        self.formulas.set_time(adj_time);
        for y in 0..image.height {
            for x in 0..image.width {
                let value = self.formulas.get_value(
                    scale_mul*(x as f32) + scale_x_offset,
                    scale_mul*(y as f32) + scale_y_offset
                );
                let color = if self.dithering {
                    self.color_mapper.get_dithered_color(value, x, y)
                } else {
                    self.color_mapper.get_nearest_color(value)
                };
                image.plot(x, y, color);
            }
        }
    }

    pub fn get_palette(&self) -> Vec<Color> {
        self.color_mapper.get_palette()
    }
}
