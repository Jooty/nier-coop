use crate::memory::MemoryError;

pub trait GameMemory {
    fn module_base(&self) -> u64;

    fn read(&self, address: u64, buffer: &mut [u8]) -> Result<(), MemoryError>;

    fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryError>;

    fn read_array<const N: usize>(&self, address: u64) -> Result<[u8; N], MemoryError> {
        let mut bytes = [0_u8; N];
        self.read(address, &mut bytes)?;
        Ok(bytes)
    }

    fn read_u8(&self, address: u64) -> Result<u8, MemoryError> {
        Ok(self.read_array::<1>(address)?[0])
    }

    fn read_i32(&self, address: u64) -> Result<i32, MemoryError> {
        Ok(i32::from_le_bytes(self.read_array(address)?))
    }

    fn read_u64(&self, address: u64) -> Result<u64, MemoryError> {
        Ok(u64::from_le_bytes(self.read_array(address)?))
    }

    fn read_f32(&self, address: u64) -> Result<f32, MemoryError> {
        Ok(f32::from_le_bytes(self.read_array(address)?))
    }

    fn read_f64(&self, address: u64) -> Result<f64, MemoryError> {
        Ok(f64::from_le_bytes(self.read_array(address)?))
    }

    fn write_u8(&mut self, address: u64, value: u8) -> Result<(), MemoryError> {
        self.write(address, &[value])
    }

    fn write_i32(&mut self, address: u64, value: i32) -> Result<(), MemoryError> {
        self.write(address, &value.to_le_bytes())
    }

    fn write_f32(&mut self, address: u64, value: f32) -> Result<(), MemoryError> {
        self.write(address, &value.to_le_bytes())
    }

    fn write_f64(&mut self, address: u64, value: f64) -> Result<(), MemoryError> {
        self.write(address, &value.to_le_bytes())
    }
}
