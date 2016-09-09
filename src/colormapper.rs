use fastmath::FastMath;
use genetics::{Chromosome, Gene};
use gradient::{Color, ControlPoint, Gradient};

const LOOKUP_TABLE_SIZE: usize = 256;

impl Color {
    fn from_hsv(hue: f32, saturation: f32, value: f32) -> Color {
        let h = hue.wrap();
        let s = saturation.clamp(0.0, 1.0);
        let v = value.clamp(0.0, 1.0);

        let sector = (h*6.0).floor() % 6.0;
        let offset = (h*6.0).fract();
        let upper = v;
        let lower = v*(1.0 - s);
        let (r, g, b) = match sector {
            0.0 => (upper, lower.lerp(upper, offset), lower),
            1.0 => (upper.lerp(lower, offset), upper, lower),
            2.0 => (lower, upper, lower.lerp(upper, offset)),
            3.0 => (lower, upper.lerp(lower, offset), upper),
            4.0 => (lower.lerp(upper, offset), lower, upper),
            5.0 => (upper, lower, upper.lerp(lower, offset)),
            _ => panic!("Invalid sector value {}", sector)
        };

        Color::new(
            (r*255.0).round() as u8,
            (g*255.0).round() as u8,
            (b*255.0).round() as u8
        )
    }
}

impl ControlPoint {
    fn from_gene(gene: &Gene) -> Option<ControlPoint> {
        assert!(gene.data.len() == 5);
        let activation_threshold = 160;
        if gene.data[0] > activation_threshold {
            let h = (gene.data[1] as f32)/256.0; // disallow h = 1.0 (wraps to 0.0)
            let s = (gene.data[2] as f32)/255.0; // allow s = 1.0
            let v = (gene.data[3] as f32)/255.0; // allow v = 1.0
            let position = (gene.data[4] as f32)/256.0; // disallow position = 1.0 (wraps to 0.0)
            Some(ControlPoint {
                color: Color::from_hsv(h, s, v),
                position: position
            })
        } else {
            None
        }
    }
}

pub struct ColorMapper {
    lookup_table: [Color; LOOKUP_TABLE_SIZE]
}

impl ColorMapper {
    pub fn new(chromosome: &Chromosome) -> ColorMapper {
        let mut lookup_table = [Color {r:0, g:0, b:0}; LOOKUP_TABLE_SIZE];
        let mut control_points = vec![];
        for gene in chromosome.genes.iter() {
            if let Some(cp) = ControlPoint::from_gene(&gene) {
                control_points.push(cp);
            }
        }
        let gradient = Gradient::new(control_points);
        let mut iter = gradient.iter();
        let mut subgradient = iter.next().unwrap();
        for i in 0..LOOKUP_TABLE_SIZE {
             let position = (i as f32)/(LOOKUP_TABLE_SIZE as f32);
             while !subgradient.contains(position) {
                 subgradient = iter.next().unwrap();
             }
             lookup_table[i] = subgradient.color_at(position);
        }

        ColorMapper {
            lookup_table: lookup_table
        }
    }

    pub fn convert(&self, value: f32) -> Color {
        let index = (value.wrap()*(LOOKUP_TABLE_SIZE as f32)).floor() as usize % LOOKUP_TABLE_SIZE;
        self.lookup_table[index]
    }
}

#[cfg(test)]
mod tests {
    use genetics::Gene;
    use gradient::Color;
    use gradient::ControlPoint;

    #[test]
    fn test_color_from_hsv() {
        /*
         * H: red -> green -> blue
         * S: white -> color
         * V: black -> color/white
         */
        // Test value = 0
        assert_eq!(Color::from_hsv(0.0, 0.0, 0.0), Color::new(0, 0, 0));
        assert_eq!(Color::from_hsv(0.0, 1.0, 0.0), Color::new(0, 0, 0));
        assert_eq!(Color::from_hsv(0.5, 0.0, 0.0), Color::new(0, 0, 0));
        assert_eq!(Color::from_hsv(0.5, 1.0, 0.0), Color::new(0, 0, 0));

        // Test parts of color wheel
        assert_eq!(Color::from_hsv(0.0,       1.0, 1.0), Color::new(255,   0,   0));
        assert_eq!(Color::from_hsv(1.0/18.0,  1.0, 1.0), Color::new(255,  85,   0));
        assert_eq!(Color::from_hsv(3.0/18.0,  1.0, 1.0), Color::new(255, 255,   0));
        assert_eq!(Color::from_hsv(4.0/18.0,  1.0, 1.0), Color::new(170, 255,   0));
        assert_eq!(Color::from_hsv(6.0/18.0,  1.0, 1.0), Color::new(  0, 255,   0));
        assert_eq!(Color::from_hsv(7.0/18.0,  1.0, 1.0), Color::new(  0, 255,  85));
        assert_eq!(Color::from_hsv(9.0/18.0,  1.0, 1.0), Color::new(  0, 255, 255));
        assert_eq!(Color::from_hsv(10.0/18.0, 1.0, 1.0), Color::new(  0, 170, 255));
        assert_eq!(Color::from_hsv(12.0/18.0, 1.0, 1.0), Color::new(  0,   0, 255));
        assert_eq!(Color::from_hsv(13.0/18.0, 1.0, 1.0), Color::new( 85,   0, 255));
        assert_eq!(Color::from_hsv(15.0/18.0, 1.0, 1.0), Color::new(255,   0, 255));
        assert_eq!(Color::from_hsv(16.0/18.0, 1.0, 1.0), Color::new(255,   0, 170));
        assert_eq!(Color::from_hsv(1.0,       1.0, 1.0), Color::new(255,   0,   0));

        // Test saturation
        assert_eq!(Color::from_hsv(0.0, 1.0, 1.0),      Color::new(255, 0, 0));
        assert_eq!(Color::from_hsv(0.0, 2.0/3.0, 1.0),  Color::new(255, 85, 85));
        assert_eq!(Color::from_hsv(0.0, 1.0/3.0, 1.0),  Color::new(255, 170, 170));
    }

    // Make sure max/min byte values map to different hues
    #[test]
    fn test_from_gene_hue() {
        let g1 = Gene { data: vec![255, 255, 255, 255, 255] };
        let g2 = Gene { data: vec![255,   0, 255, 255, 255] };
        let cp1 = ControlPoint::from_gene(&g1).unwrap();
        let cp2 = ControlPoint::from_gene(&g2).unwrap();
        assert!(cp1.color != cp2.color); // Make sure we have different hues
    }

    // Make sure full range of saturation is possible
    #[test]
    fn test_from_gene_saturation() {
        let g1 = Gene { data: vec![255, 0, 255, 255, 255] };
        let g2 = Gene { data: vec![255, 0,   0, 255, 255] };
        let cp1 = ControlPoint::from_gene(&g1).unwrap();
        let cp2 = ControlPoint::from_gene(&g2).unwrap();
        assert_eq!(cp1.color, Color::from_hsv(0.0, 1.0, 1.0));
        assert_eq!(cp2.color, Color::from_hsv(0.0, 0.0, 1.0));
    }

    // Make sure full range of value is possible
    #[test]
    fn test_from_gene_value() {
        let g1 = Gene { data: vec![255, 0, 255, 255, 255] };
        let g2 = Gene { data: vec![255, 0, 255,   0, 255] };
        let cp1 = ControlPoint::from_gene(&g1).unwrap();
        let cp2 = ControlPoint::from_gene(&g2).unwrap();
        assert_eq!(cp1.color, Color::from_hsv(0.0, 1.0, 1.0));
        assert_eq!(cp2.color, Color::from_hsv(0.0, 1.0, 0.0));
    }

    // Make sure max/min byte values map to different positions
    #[test]
    fn test_from_gene_position() {
        let g1 = Gene { data: vec![255, 255, 255, 255, 255] };
        let g2 = Gene { data: vec![255, 255, 255, 255,   0] };
        let cp1 = ControlPoint::from_gene(&g1).unwrap();
        let cp2 = ControlPoint::from_gene(&g2).unwrap();
        assert!(cp1.position != cp2.position);
    }
}
