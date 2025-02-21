use image::{DynamicImage, GenericImageView};
use regex::Regex;
use std::io::Write;
use std::path::PathBuf;
use std::{error::Error, process::Command, str, str::FromStr};

const VENDOR_ID: u16 = 1452;
const PRODUCT_ID: u16 = 4617;

pub fn search_ipod() -> Option<String> {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if VENDOR_ID == device_desc.vendor_id() && PRODUCT_ID == device_desc.product_id() {
            return get_ipod_path();
        }
    }
    None
}

fn list() -> Result<Vec<String>, Box<dyn Error>> {
    let mut disks = Vec::new();
    let r = match Command::new("diskutil").arg("list").output() {
        Ok(s) => s,
        Err(e) => return Err(Box::new(e)),
    };
    if !r.status.success() {
        return Ok(disks);
    }
    let rg = Regex::new(r"\d:.+   [a-zA-Z0-9].+").unwrap();
    let a = match str::from_utf8(&r.stdout) {
        Ok(r) => r,
        Err(e) => return Err(Box::new(e)),
    };
    for cap in Regex::new(r"\/dev\/.+\(external\, physical\):")
        .unwrap()
        .find_iter(a)
    {
        let mut b = &a[cap.end()..];
        let i = match b.find("\n\n") {
            Some(r) => r,
            None => return Ok(disks),
        };
        b = &b[..i];
        for gap in rg.find_iter(b) {
            let j = match gap.as_str().rfind(" ") {
                Some(r) => r + 1,
                None => return Ok(disks),
            };

            let g = &gap.as_str()[j..];
            disks.push(String::from_str(g).unwrap());
        }
    }
    Ok(disks)
}

fn is_ipod(name: &str) -> bool {
    let r = match Command::new("diskutil").arg("info").arg(name).output() {
        Ok(s) => s,
        Err(_e) => return false,
    };
    if !r.status.success() {
        return false;
    }
    let a = match str::from_utf8(&r.stdout) {
        Ok(r) => r,
        Err(_e) => return false,
    };
    let cap = Regex::new(r"Media Type:.+\n").unwrap().find(a);
    if let Some(g) = cap {
        let mut b = g.as_str();
        let f = b.rfind(" ").unwrap() + 1;
        b = &b[f..b.len() - 1];
        return b == "iPod";
    }
    false
}

fn get_mount_point(name: &str) -> Option<String> {
    let r = match Command::new("diskutil").arg("info").arg(name).output() {
        Ok(s) => s,
        Err(_e) => return None,
    };
    if !r.status.success() {
        return None;
    }
    let a = match str::from_utf8(&r.stdout) {
        Ok(r) => r,
        Err(_e) => return None,
    };
    let cap = Regex::new(r"Mount Point:.+\n").unwrap().find(a);
    match cap {
        Some(g) => {
            let i = g.as_str();
            let j = i.rfind(" ").unwrap() + 1;
            Some(i[j..i.len() - 1].to_string())
        }
        None => None,
    }
}

fn get_ipod_path() -> Option<String> {
    match list() {
        Ok(l) => l
            .iter()
            .filter(|d| is_ipod(d))
            .filter_map(|d| get_mount_point(d))
            .last(),
        Err(_e) => None,
    }
}

pub struct IPodImage {
    pixels: Vec<u16>,
}

impl IPodImage {
    pub fn write(&self, p: PathBuf) {
        let mut file = std::fs::File::create(p).unwrap();
        let _ = file.write(&self.convert_to_u8());
    }

    fn convert_to_u8(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|f| [*f as u8, (*f >> 8) as u8])
            .collect()
    }
}

impl From<DynamicImage> for IPodImage {
    fn from(value: DynamicImage) -> Self {
        let img_rgba = value.to_rgba8();

        let (width, height) = img_rgba.dimensions();

        let mut rgb565_data: Vec<u16> = Vec::new();

        for y in 0..height {
            for x in 0..width {
                let pixel = img_rgba.get_pixel(x, y).0;

                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];

                rgb565_data.push(rgb_to_rgb565(r, g, b));
            }
        }

        Self {
            pixels: rgb565_data,
        }
    }
}

fn rgb_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r_565 = (r >> 3) & 0x1F; // Extract top 5 bits
    let g_565 = (g >> 2) & 0x3F; // Extract top 6 bits
    let b_565 = (b >> 3) & 0x1F; // Extract top 5 bits

    ((r_565 as u16) << 11) | ((g_565 as u16) << 5) | (b_565 as u16) // Combine to RGB565
}
