use ferrosa_sim::ComputerMode;
use wasmtime::component::Resource;

use super::PlayerRef;
use crate::{
    bindings::ferrosa::plugin::types::{self, ScreenColour},
    host::HostState,
};

pub struct ComputerRef(pub(crate) usize);

fn packed(colour: ScreenColour) -> u8 {
    (shade(colour.background) << 4) | shade(colour.foreground)
}

fn shade(shade: types::ScreenShade) -> u8 {
    shade as u8 & 0xf
}

fn mode(mode: ComputerMode) -> types::ComputerMode {
    match mode {
        ComputerMode::Shell => types::ComputerMode::Shell,
        ComputerMode::Decrypt => types::ComputerMode::Decrypt,
        ComputerMode::Mission => types::ComputerMode::Mission,
        ComputerMode::Plugin => types::ComputerMode::Plugin,
    }
}

impl HostState {
    pub(crate) fn computer_handle(&mut self, item: usize) -> wasmtime::Result<Resource<ComputerRef>> {
        Ok(self.table.push(ComputerRef(item))?)
    }

    fn item_of(&mut self, computer: &Resource<ComputerRef>) -> wasmtime::Result<usize> {
        Ok(self.table.get(computer)?.0)
    }
}

impl types::HostComputer for HostState {
    fn item(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<u32> {
        Ok(self.item_of(&computer)? as u32)
    }

    fn exists(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<bool> {
        let item = self.item_of(&computer)?;
        Ok(self.sim()?.is_computer(item))
    }

    fn holder(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<Option<Resource<PlayerRef>>> {
        let item = self.item_of(&computer)?;
        let sim = self.sim()?;
        let Some(player) = sim.item_holder(item).and_then(|h| sim.human(h)).and_then(|h| h.player) else { return Ok(None) };

        self.player_handle(player).map(Some)
    }

    fn team(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<Option<types::Team>> {
        let item = self.item_of(&computer)?;
        Ok(self.sim()?.computer_team(item).and_then(|t| match t {
            0 => Some(types::Team::Goldmen),
            1 => Some(types::Team::Monsota),
            2 => Some(types::Team::Oxs),
            3 => Some(types::Team::Nexaco),
            4 => Some(types::Team::Pentacom),
            5 => Some(types::Team::Prodocon),
            _ => None,
        }))
    }

    fn mode(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<types::ComputerMode> {
        let item = self.item_of(&computer)?;
        Ok(self.sim()?.computer_mode(item).map_or(types::ComputerMode::Shell, mode))
    }

    fn program(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<Option<String>> {
        let item = self.item_of(&computer)?;
        Ok(self.sim()?.computer_program(item).map(str::to_string))
    }

    fn print(&mut self, computer: Resource<ComputerRef>, text: String) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        self.sim_mut()?.computer_print(item, &text);
        Ok(())
    }

    fn write_line(&mut self, computer: Resource<ComputerRef>, line: u32, text: String, colour: ScreenColour) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        self.sim_mut()?.computer_write_line(item, line, &text, packed(colour));
        Ok(())
    }

    fn write_field(&mut self, computer: Resource<ComputerRef>, line: u32, text: String, width: u32, colour: ScreenColour) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;

        self.sim_mut()?.computer_write_field(item, line, &text, width, packed(colour));
        Ok(())
    }

    fn write_cells(&mut self, computer: Resource<ComputerRef>, line: u32, text: String, colours: Vec<ScreenColour>) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        let colours: Vec<u8> = colours.into_iter().map(packed).collect();

        self.sim_mut()?.computer_write_cells(item, line, &text, &colours);
        Ok(())
    }

    fn read_line(&mut self, computer: Resource<ComputerRef>, line: u32) -> wasmtime::Result<Option<String>> {
        let item = self.item_of(&computer)?;
        Ok(self.sim()?.computer_read_line(item, line))
    }

    fn clear(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        self.sim_mut()?.computer_clear(item);
        Ok(())
    }

    fn set_cursor(&mut self, computer: Resource<ComputerRef>, at: Option<(u32, u32)>) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        self.sim_mut()?.computer_set_cursor(item, at);
        Ok(())
    }

    fn exit(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<()> {
        let item = self.item_of(&computer)?;
        self.sim_mut()?.exit_program(item);
        Ok(())
    }

    fn drive_volume(&mut self, computer: Resource<ComputerRef>, drive: u32) -> wasmtime::Result<Option<Resource<VolumeRef>>> {
        let item = self.item_of(&computer)?;
        let Some(volume) = self.sim()?.computer_drive_volume(item, drive) else { return Ok(None) };

        self.volume_handle(volume).map(Some)
    }

    fn drop(&mut self, computer: Resource<ComputerRef>) -> wasmtime::Result<()> {
        self.table.delete(computer)?;
        Ok(())
    }
}

pub struct VolumeRef(pub(crate) i32);

pub struct FileRef(pub(crate) i32, pub(crate) usize);

impl HostState {
    pub(crate) fn volume_handle(&mut self, volume: i32) -> wasmtime::Result<Resource<VolumeRef>> {
        Ok(self.table.push(VolumeRef(volume))?)
    }

    fn file_handle(&mut self, volume: i32, file: usize) -> wasmtime::Result<Resource<FileRef>> {
        Ok(self.table.push(FileRef(volume, file))?)
    }
}

impl types::HostVolume for HostState {
    fn id(&mut self, volume: Resource<VolumeRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&volume)?.0 as u32)
    }

    fn capacity(&mut self, volume: Resource<VolumeRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&volume)?.0;
        Ok(self.sim()?.volume_capacity(id).unwrap_or(0))
    }

    fn files(&mut self, volume: Resource<VolumeRef>) -> wasmtime::Result<Vec<Resource<FileRef>>> {
        let id = self.table.get(&volume)?.0;
        let count = self.sim()?.volume_files(id);

        (0..count).map(|f| self.file_handle(id, f)).collect()
    }

    fn add_file(&mut self, volume: Resource<VolumeRef>, name: String, text: String) -> wasmtime::Result<Option<Resource<FileRef>>> {
        let id = self.table.get(&volume)?.0;
        let Some(file) = self.sim_mut()?.add_volume_file(id, &name, &text) else { return Ok(None) };

        self.file_handle(id, file).map(Some)
    }

    fn drop(&mut self, volume: Resource<VolumeRef>) -> wasmtime::Result<()> {
        self.table.delete(volume)?;
        Ok(())
    }
}

impl types::HostFile for HostState {
    fn name(&mut self, file: Resource<FileRef>) -> wasmtime::Result<String> {
        let FileRef(volume, file) = *self.table.get(&file)?;
        Ok(self.sim()?.file_name(volume, file).unwrap_or_default())
    }

    fn text(&mut self, file: Resource<FileRef>) -> wasmtime::Result<String> {
        let FileRef(volume, file) = *self.table.get(&file)?;
        Ok(self.sim()?.file_text(volume, file).unwrap_or_default())
    }

    fn set_text(&mut self, file: Resource<FileRef>, text: String) -> wasmtime::Result<()> {
        let FileRef(volume, file) = *self.table.get(&file)?;
        self.sim_mut()?.set_file_text(volume, file, &text);
        Ok(())
    }

    fn encrypted(&mut self, file: Resource<FileRef>) -> wasmtime::Result<bool> {
        let FileRef(volume, file) = *self.table.get(&file)?;
        Ok(self.sim()?.file_encrypted(volume, file))
    }

    fn set_encrypted(&mut self, file: Resource<FileRef>, encrypted: bool) -> wasmtime::Result<()> {
        let FileRef(volume, file) = *self.table.get(&file)?;
        self.sim_mut()?.set_file_encrypted(volume, file, encrypted);
        Ok(())
    }

    fn drop(&mut self, file: Resource<FileRef>) -> wasmtime::Result<()> {
        self.table.delete(file)?;
        Ok(())
    }
}
