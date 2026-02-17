use ply_rs_bw::PlyRead;

#[derive(Copy, Clone)]
struct NotDefault(u8);

impl ply_rs_bw::ply::SetProperty<u8> for NotDefault {
    fn set(&mut self, val: u8) {
        self.0 = val;
    }
}

impl ply_rs_bw::ply::GetProperty<u8> for NotDefault {
    fn get(&self) -> Option<u8> {
        Some(self.0)
    }
}

#[derive(PlyRead)]
struct Bad {
    #[ply(type = "u8")]
    a: NotDefault,
}

fn main() {}
