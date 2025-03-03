/*
 * This file is part of the oreboot project.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; version 2 of the License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 */

use consts::DeviceCtl;
use core::ops;
use model::*;

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_bitfields;
use tock_registers::registers::{ReadOnly, ReadWrite};

const RETRY_COUNT: u32 = 100_000;
const UART0: usize = 0xfedc9000;
const UART1: usize = 0xfedca000;

// We fill out as little of this as possible.
// We're firmware and should never plan to use it
// with all its features.
#[repr(C)]
pub struct RegisterBlock {
    // pathetically, this is also the DLL.
    d: ReadWrite<u32, D::Register>, /* data register */
    // This is the IER when we are not in DLAB mode.
    // We never intend to set inerrupts, so we just set
    // it up as thing we write to.
    dlm: ReadWrite<u32, D::Register>, /* data register */
    _8: u32,
    lcr: ReadWrite<u32, LCR::Register>,
    _10: u32,
    stat: ReadOnly<u32, STAT::Register>, /* status */
}

pub struct UART {
    base: usize,
    // TODO: implement baudrate
    //baudrate: u32,
}

impl ops::Deref for UART {
    type Target = RegisterBlock;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

register_bitfields![
    u32,
    D [
        Data OFFSET(0) NUMBITS(8) []
    ],
    STAT[
        DR OFFSET(0) NUMBITS(1) [],
        THRE OFFSET(5) NUMBITS(1) [],
        TEMT OFFSET(6) NUMBITS(1) []
    ],
    FCR[
        FIFOENABLE OFFSET(0) NUMBITS(1) []
    ],
    LCR[
        BITSPARITY OFFSET(0) NUMBITS(3) [
            EIGHTN1 = 3
        ],
        DLAB OFFSET(7) NUMBITS(1)[
            Data = 0,
            BaudRate = 1
        ]
    ]
];

impl UART {
    /// # Safety
    /// This could alias any other reference, and also it's losing provenance information
    pub unsafe fn new(base: usize) -> UART {
        Self { base }
    }

    pub fn uart0() -> UART {
        Self { base: UART0 }
    }

    pub fn uart1() -> UART {
        Self { base: UART1 }
    }

    /// Returns a pointer to the register block
    fn ptr(&self) -> *const RegisterBlock {
        self.base as *const _
    }
}

impl Driver for UART {
    fn init(&mut self) -> Result<()> {
        self.lcr
            .write(LCR::BITSPARITY::EIGHTN1 + LCR::DLAB::BaudRate);
        self.dlm.set(0);
        self.d.set(1); // approx. 115200
        self.lcr.write(LCR::BITSPARITY::EIGHTN1 + LCR::DLAB::Data);
        self.dlm.set(0); // Just clear the IER to be safe.
        Ok(())
    }

    fn pread(&self, data: &mut [u8], _offset: usize) -> Result<usize> {
        'outer: for (read_count, c) in data.iter_mut().enumerate() {
            for _ in 0..RETRY_COUNT {
                let stat = self.stat.extract();
                if stat.is_set(STAT::DR) {
                    let d = self.d.extract();
                    *c = d.read(D::Data) as u8;
                    continue 'outer;
                }
            }
            return Ok(read_count);
        }
        Ok(data.len())
    }

    fn pwrite(&mut self, data: &[u8], _offset: usize) -> Result<usize> {
        'outer: for (sent_count, &c) in data.iter().enumerate() {
            for _ in 0..RETRY_COUNT {
                if self.stat.is_set(STAT::THRE) {
                    self.d.set(c.into());
                    continue 'outer;
                }
            }
            return Ok(sent_count);
        }
        Ok(data.len())
    }

    fn ctl(&mut self, __d: DeviceCtl) -> Result<usize> {
        NOT_IMPLEMENTED
    }

    fn stat(&self, _data: &mut [u8]) -> Result<usize> {
        NOT_IMPLEMENTED
    }

    fn shutdown(&mut self) {}
}
