use byteorder::{BigEndian, WriteBytesExt};
use {binary, Coil, Error, Function, Result};

pub trait Client {
    fn read_function_result(self: &mut Self, fun: &Function) -> Result<Vec<u8>>;
    fn write(self: &mut Self, buff: &[u8]) -> Result<()>;

    /// Read `count` bits starting at address `addr`.
    fn read_coils(self: &mut Self, addr: u16, count: u16) -> Result<Vec<Coil>> {
        let bytes = self.read_function_result(&Function::ReadCoils(addr, count))?;
        Ok(binary::unpack_bits(&bytes, count))
    }

    /// Read `count` input bits starting at address `addr`.
    fn read_discrete_inputs(self: &mut Self, addr: u16, count: u16) -> Result<Vec<Coil>> {
        let bytes = self.read_function_result(&Function::ReadDiscreteInputs(addr, count))?;
        Ok(binary::unpack_bits(&bytes, count))
    }

    /// Read `count` 16bit input registers starting at address `addr`.
    fn read_input_registers(self: &mut Self, addr: u16, count: u16) -> Result<Vec<u16>> {
        let bytes = self.read_function_result(&Function::ReadInputRegisters(addr, count))?;
        binary::pack_bytes(&bytes[..])
    }

    /// Read `count` 16bit registers starting at address `addr`.
    fn read_holding_registers(self: &mut Self, addr: u16, count: u16) -> Result<Vec<u16>> {
        let bytes = self.read_function_result(&Function::ReadHoldingRegisters(addr, count))?;
        binary::pack_bytes(&bytes[..])
    }

    fn write_single(self: &mut Self, fun: &Function) -> Result<()> {
        let (addr, value) = match *fun {
            Function::WriteSingleCoil(a, v) | Function::WriteSingleRegister(a, v) => (a, v),
            _ => return Err(Error::InvalidFunction),
        };

        let mut buff = Vec::new();
        buff.write_u8(fun.code())?;
        buff.write_u16::<BigEndian>(addr)?;
        buff.write_u16::<BigEndian>(value)?;
        self.write(&buff)
    }

    fn write_multiple(self: &mut Self, fun: &Function) -> Result<()> {
        let (addr, quantity, values) = match *fun {
            Function::WriteMultipleCoils(a, q, v) | Function::WriteMultipleRegisters(a, q, v) => {
                (a, q, v)
            }
            _ => return Err(Error::InvalidFunction),
        };

        let mut buff = Vec::new();
        buff.write_u8(fun.code())?;
        buff.write_u16::<BigEndian>(addr)?;
        buff.write_u16::<BigEndian>(quantity)?;
        buff.write_u8(values.len() as u8)?;
        for v in values {
            buff.write_u8(*v)?;
        }
        self.write(&buff)
    }

    /// Write a single coil (bit) to address `addr`.
    fn write_single_coil(self: &mut Self, addr: u16, value: Coil) -> Result<()> {
        self.write_single(&Function::WriteSingleCoil(addr, value.code()))
    }

    /// Write a single 16bit register to address `addr`.
    fn write_single_register(self: &mut Self, addr: u16, value: u16) -> Result<()> {
        println!("Write single register");
        self.write_single(&Function::WriteSingleRegister(addr, value))
    }

    /// Write a multiple coils (bits) starting at address `addr`.
    fn write_multiple_coils(self: &mut Self, addr: u16, values: &[Coil]) -> Result<()> {
        let bytes = binary::pack_bits(values);
        self.write_multiple(&Function::WriteMultipleCoils(
            addr,
            values.len() as u16,
            &bytes,
        ))
    }

    /// Write a multiple 16bit registers starting at address `addr`.
    fn write_multiple_registers(self: &mut Self, addr: u16, values: &[u16]) -> Result<()> {
        let bytes = binary::unpack_bytes(values);
        self.write_multiple(&Function::WriteMultipleRegisters(
            addr,
            values.len() as u16,
            &bytes,
        ))
    }
}
