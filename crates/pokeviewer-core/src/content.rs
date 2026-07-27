//! Allocation-free content-pack validation and lookup.

const HEADER_LENGTH: usize = 32;
const RECORD_LENGTH: usize = 6;
const RECORD_COUNT: usize = 151;
const SPRITE_WIDTH: usize = 56;
const SPRITE_HEIGHT: usize = 56;
const NO_SECONDARY_TYPE: u8 = 0xff;

/// Bytes in one 56 × 56 one-bit content sprite.
pub const CONTENT_SPRITE_BYTES: usize = SPRITE_WIDTH * SPRITE_HEIGHT / 8;

/// Stable Pokémon type codes stored in content-pack v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PokemonType {
    /// Normal.
    Normal = 0,
    /// Fire.
    Fire = 1,
    /// Water.
    Water = 2,
    /// Electric.
    Electric = 3,
    /// Grass.
    Grass = 4,
    /// Ice.
    Ice = 5,
    /// Fighting.
    Fighting = 6,
    /// Poison.
    Poison = 7,
    /// Ground.
    Ground = 8,
    /// Flying.
    Flying = 9,
    /// Psychic.
    Psychic = 10,
    /// Bug.
    Bug = 11,
    /// Rock.
    Rock = 12,
    /// Ghost.
    Ghost = 13,
    /// Dragon.
    Dragon = 14,
    /// Dark.
    Dark = 15,
    /// Steel.
    Steel = 16,
    /// Fairy.
    Fairy = 17,
}

impl PokemonType {
    fn from_code(code: u8) -> Result<Self, PackError> {
        match code {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Fire),
            2 => Ok(Self::Water),
            3 => Ok(Self::Electric),
            4 => Ok(Self::Grass),
            5 => Ok(Self::Ice),
            6 => Ok(Self::Fighting),
            7 => Ok(Self::Poison),
            8 => Ok(Self::Ground),
            9 => Ok(Self::Flying),
            10 => Ok(Self::Psychic),
            11 => Ok(Self::Bug),
            12 => Ok(Self::Rock),
            13 => Ok(Self::Ghost),
            14 => Ok(Self::Dragon),
            15 => Ok(Self::Dark),
            16 => Ok(Self::Steel),
            17 => Ok(Self::Fairy),
            _ => Err(PackError::InvalidType),
        }
    }
}

/// A fully validated, borrowed Pokémon record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PokemonRecord<'a> {
    /// National Pokédex ID.
    pub dex_id: u8,
    /// NFC-normalized English display name.
    pub name: &'a str,
    /// Current canonical primary type.
    pub primary_type: PokemonType,
    /// Current canonical secondary type, when present.
    pub secondary_type: Option<PokemonType>,
    /// Borrowed 56 × 56, row-major, one-bit sprite.
    pub sprite: &'a [u8; CONTENT_SPRITE_BYTES],
}

/// A validated v1 pack backed directly by its flash bytes.
#[derive(Clone, Copy, Debug)]
pub struct ContentPack<'a> {
    records: &'a [u8],
    names: &'a [u8],
    schedule: &'a [u8],
    sprites: &'a [u8],
}

/// Bounded failure codes for incompatible or corrupt content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackError {
    /// Header or section length is invalid.
    InvalidLength,
    /// Header magic is not `PKVW`.
    InvalidMagic,
    /// Format, content, or schedule version is unsupported.
    UnsupportedVersion,
    /// A fixed header field does not match content-pack v1.
    InvalidHeader,
    /// Payload checksum does not match.
    InvalidChecksum,
    /// Records are missing, duplicated, or out of order.
    InvalidRecordOrder,
    /// A record contains an unsupported type code.
    InvalidType,
    /// A name offset, length, encoding, or character is invalid.
    InvalidName,
    /// Schedule v1 is missing, duplicated, or changed.
    InvalidSchedule,
    /// Requested Pokédex ID or cycle index is outside v1.
    OutOfRange,
}

impl<'a> ContentPack<'a> {
    /// Validate and borrow a complete content-pack v1 image.
    ///
    /// # Errors
    ///
    /// Returns a bounded [`PackError`] for any incompatible or corrupt field.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, PackError> {
        if bytes.len() < HEADER_LENGTH {
            return Err(PackError::InvalidLength);
        }
        if bytes.get(0..4) != Some(b"PKVW") {
            return Err(PackError::InvalidMagic);
        }
        if read_u16(bytes, 4)? != 1 || read_u32(bytes, 8)? != 1 || read_u16(bytes, 12)? != 1 {
            return Err(PackError::UnsupportedVersion);
        }
        if usize::from(read_u16(bytes, 6)?) != HEADER_LENGTH
            || usize::from(read_u16(bytes, 14)?) != RECORD_COUNT
            || usize::from(read_u16(bytes, 16)?) != RECORD_COUNT
            || bytes[18] != 56
            || bytes[19] != 56
            || usize::from(bytes[20]) != RECORD_LENGTH
            || bytes[21] != 0
        {
            return Err(PackError::InvalidHeader);
        }

        let payload = bytes.get(HEADER_LENGTH..).ok_or(PackError::InvalidLength)?;
        if usize::try_from(read_u32(bytes, 24)?).ok() != Some(payload.len()) {
            return Err(PackError::InvalidLength);
        }
        if crc32fast::hash(payload) != read_u32(bytes, 28)? {
            return Err(PackError::InvalidChecksum);
        }

        let records_length = RECORD_COUNT * RECORD_LENGTH;
        let names_length = usize::from(read_u16(bytes, 22)?);
        let schedule_end = records_length
            .checked_add(names_length)
            .and_then(|value| value.checked_add(RECORD_COUNT))
            .ok_or(PackError::InvalidLength)?;
        let expected_payload = schedule_end
            .checked_add(RECORD_COUNT * CONTENT_SPRITE_BYTES)
            .ok_or(PackError::InvalidLength)?;
        if payload.len() != expected_payload {
            return Err(PackError::InvalidLength);
        }

        let records = &payload[..records_length];
        let names_end = records_length + names_length;
        let names = &payload[records_length..names_end];
        let schedule = &payload[names_end..schedule_end];
        let sprites = &payload[schedule_end..];
        let pack = Self {
            records,
            names,
            schedule,
            sprites,
        };
        pack.validate_records()?;
        pack.validate_schedule()?;
        Ok(pack)
    }

    /// Borrow a record by National Pokédex ID.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::OutOfRange`] unless `dex_id` is 1–151.
    pub fn record(&self, dex_id: u8) -> Result<PokemonRecord<'a>, PackError> {
        if !(1..=151).contains(&dex_id) {
            return Err(PackError::OutOfRange);
        }
        self.record_at(usize::from(dex_id - 1))
    }

    /// Resolve schedule v1 and borrow its record.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::OutOfRange`] unless `cycle_index` is 0–150.
    pub fn scheduled_record(&self, cycle_index: u8) -> Result<PokemonRecord<'a>, PackError> {
        let dex_id = *self
            .schedule
            .get(usize::from(cycle_index))
            .ok_or(PackError::OutOfRange)?;
        self.record(dex_id)
    }

    fn validate_records(&self) -> Result<(), PackError> {
        let mut expected_name_offset = 0;
        for index in 0..RECORD_COUNT {
            let record = self.record_at(index)?;
            if usize::from(record.dex_id) != index + 1 {
                return Err(PackError::InvalidRecordOrder);
            }
            let bytes = self.record_bytes(index)?;
            let name_offset = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
            if name_offset != expected_name_offset {
                return Err(PackError::InvalidName);
            }
            expected_name_offset += record.name.len();
        }
        if expected_name_offset != self.names.len() {
            return Err(PackError::InvalidName);
        }
        Ok(())
    }

    fn validate_schedule(&self) -> Result<(), PackError> {
        for (index, actual) in self.schedule.iter().copied().enumerate() {
            let expected = u8::try_from((73 * index) % RECORD_COUNT + 1)
                .map_err(|_| PackError::InvalidSchedule)?;
            if actual != expected {
                return Err(PackError::InvalidSchedule);
            }
        }
        Ok(())
    }

    fn record_at(&self, index: usize) -> Result<PokemonRecord<'a>, PackError> {
        let bytes = self.record_bytes(index)?;
        let primary_type = PokemonType::from_code(bytes[1])?;
        let secondary_type = if bytes[2] == NO_SECONDARY_TYPE {
            None
        } else {
            Some(PokemonType::from_code(bytes[2])?)
        };
        if secondary_type == Some(primary_type) {
            return Err(PackError::InvalidType);
        }

        let name_length = usize::from(bytes[3]);
        if !(1..=16).contains(&name_length) {
            return Err(PackError::InvalidName);
        }
        let name_offset = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        let name_end = name_offset
            .checked_add(name_length)
            .ok_or(PackError::InvalidName)?;
        let name_bytes = self
            .names
            .get(name_offset..name_end)
            .ok_or(PackError::InvalidName)?;
        let name = core::str::from_utf8(name_bytes).map_err(|_| PackError::InvalidName)?;
        if name.chars().any(char::is_control) {
            return Err(PackError::InvalidName);
        }

        let sprite_start = index
            .checked_mul(CONTENT_SPRITE_BYTES)
            .ok_or(PackError::InvalidLength)?;
        let sprite_end = sprite_start
            .checked_add(CONTENT_SPRITE_BYTES)
            .ok_or(PackError::InvalidLength)?;
        let sprite = self
            .sprites
            .get(sprite_start..sprite_end)
            .and_then(|value| value.try_into().ok())
            .ok_or(PackError::InvalidLength)?;
        Ok(PokemonRecord {
            dex_id: bytes[0],
            name,
            primary_type,
            secondary_type,
            sprite,
        })
    }

    fn record_bytes(&self, index: usize) -> Result<&'a [u8], PackError> {
        let start = index
            .checked_mul(RECORD_LENGTH)
            .ok_or(PackError::InvalidLength)?;
        let end = start
            .checked_add(RECORD_LENGTH)
            .ok_or(PackError::InvalidLength)?;
        self.records.get(start..end).ok_or(PackError::OutOfRange)
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, PackError> {
    let end = offset.checked_add(2).ok_or(PackError::InvalidLength)?;
    bytes
        .get(offset..end)
        .and_then(|value| value.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or(PackError::InvalidLength)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, PackError> {
    let end = offset.checked_add(4).ok_or(PackError::InvalidLength)?;
    bytes
        .get(offset..end)
        .and_then(|value| value.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(PackError::InvalidLength)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{CONTENT_SPRITE_BYTES, ContentPack, PackError, RECORD_COUNT};

    const PACK: &[u8] = include_bytes!("../../../content/generated/pokeviewer-v1.pack");

    #[test]
    fn committed_pack_exposes_every_record_without_allocation() {
        let pack = ContentPack::parse(PACK).unwrap();

        for dex_id in 1..=151 {
            let record = pack.record(dex_id).unwrap();
            assert_eq!(record.dex_id, dex_id);
            assert!(!record.name.is_empty());
            assert_eq!(record.sprite.len(), CONTENT_SPRITE_BYTES);
        }
        for index in 0..RECORD_COUNT {
            let cycle_index = u8::try_from(index).unwrap();
            let record = pack.scheduled_record(cycle_index).unwrap();
            assert!((1..=151).contains(&record.dex_id));
        }
        assert!(core::mem::size_of_val(&pack) <= 64);
    }

    #[test]
    fn corrupted_payload_is_rejected_before_lookup() {
        let mut corrupted = std::vec::Vec::from(PACK);
        corrupted[32] ^= 1;

        assert_eq!(
            ContentPack::parse(&corrupted).unwrap_err(),
            PackError::InvalidChecksum
        );
    }
}
