use std::fmt::Display;

use fancy_regex::Regex;

#[repr(C)]
pub struct Freq {
    pub ch: i32,
    pub slot: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelSpace {
    pub space: u32,
    pub ch: u32,
    pub space_description: Option<String>,
    pub ch_description: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelType {
    Terrestrial(u8),
    Catv(u8),
    BS(u8, u32),
    CS(u8),
    Bon(ChannelSpace),
    Undefined,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub ch_type: ChannelType,
    pub raw_string: String,
}

impl Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        type T = ChannelType;
        match self {
            T::Terrestrial(ch) => write!(f, "GR: {}", ch),
            T::Catv(ch) => write!(f, "CATV: {}", ch),
            T::BS(ch, stream_id) => write!(f, "BS: {}-{}", ch, stream_id),
            T::CS(ch) => write!(f, "CS: {}", ch),
            _ => write!(f, "Undefined"),
        }
    }
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(Raw->{})", self.ch_type, self.raw_string)
    }
}

impl Channel {
    pub fn from_ch_str(ch_str: impl Into<String>) -> Channel {
        //TODO: Refactor
        let ch_str = ch_str.into();

        let isdb_t_regex = Regex::new(r"(?<=[TC])\d{1,2}\b").unwrap();
        let cs_regex = Regex::new(r"(?<=CS)\d?[02468]\b").unwrap();
        let bs_regex = Regex::new(r"(?<=BS)\d[13579]_[01234567]\b").unwrap();
        let bon_regex = Regex::new(r"^[0-9]+-[0-9]+$").unwrap();

        if let Ok(Some(m)) = isdb_t_regex.find(&ch_str) {
            let first_letter = ch_str.chars().nth(0).unwrap();
            let physical_ch_num = m.as_str().parse().unwrap();
            let ch_type = if first_letter == 'T' {
                ChannelType::Terrestrial(physical_ch_num)
            } else {
                ChannelType::Catv(physical_ch_num)
            };

            Channel {
                ch_type,
                raw_string: ch_str.clone(),
            }
        } else if cs_regex.is_match(&ch_str).unwrap() {
            let caps = cs_regex.captures(&ch_str).unwrap().unwrap();
            let result_str = caps.get(0).map_or("", |m| m.as_str());
            let physical_ch_num = result_str.parse().unwrap();
            let ch_type = ChannelType::CS(physical_ch_num);

            Channel {
                ch_type,
                raw_string: ch_str.clone(),
            }
        } else if bs_regex.is_match(&ch_str).unwrap() {
            let caps = bs_regex.captures(&ch_str).unwrap().unwrap();
            let result_str = caps.get(0).map_or("", |m| m.as_str());

            let ch_type = {
                let split_loc = result_str.rfind('_').unwrap();
                let physical_ch_num = (result_str[0..split_loc]).parse().unwrap();
                let stream_id: u32 = result_str[split_loc + 1..].parse().unwrap();
                ChannelType::BS(physical_ch_num, stream_id)
            };

            Channel {
                ch_type,
                raw_string: ch_str.clone(),
            }
        } else if bon_regex.is_match(&ch_str).unwrap() {
            let caps = bon_regex.captures(&ch_str).unwrap().unwrap();
            let result_str = caps.get(0).map_or("", |m| m.as_str());

            let ch_type = {
                let split_loc = result_str.rfind('-').unwrap();
                let space: u32 = result_str[0..split_loc].parse().unwrap();
                let ch: u32 = result_str[split_loc + 1..].parse().unwrap();
                ChannelType::Bon(ChannelSpace {
                    space,
                    ch,
                    space_description: None,
                    ch_description: None,
                })
            };

            Channel {
                ch_type,
                raw_string: ch_str.clone(),
            }
        } else {
            Channel {
                ch_type: ChannelType::Undefined,
                raw_string: ch_str,
            }
        }
    }
    pub fn try_get_physical_num(&self) -> Option<u8> {
        match self.ch_type {
            ChannelType::Terrestrial(ch) => Some(ch),
            ChannelType::Catv(ch) => Some(ch),
            ChannelType::BS(ch, _) => Some(ch),
            ChannelType::CS(ch) => Some(ch),
            _ => None,
        }
    }
    pub fn to_ioctl_freq(&self, freq_offset: i32) -> Freq {
        let ioctl_channel = match self.ch_type {
            ChannelType::Terrestrial(ch_num) if (13..=52).contains(&ch_num) => ch_num + 50,
            ChannelType::Catv(ch_num) if (23..=63).contains(&ch_num) => ch_num - 1,
            ChannelType::Catv(ch_num) if (13..=22).contains(&ch_num) => ch_num - 10,
            ChannelType::CS(ch_num) if (2..=24).contains(&ch_num) && (ch_num % 2 == 0) => {
                ch_num / 2 + 11
            }
            ChannelType::BS(ch_num, _) if (1..=23).contains(&ch_num) && (ch_num % 2 == 1) => {
                ch_num / 2
            }
            ChannelType::Undefined => unimplemented!(),
            _ => panic!("Invalid channel."),
        };
        let slot = match self.ch_type {
            ChannelType::CS(_) => 0,
            ChannelType::BS(_, stream_id) => stream_id as i32,
            _ => freq_offset,
        };
        Freq {
            ch: ioctl_channel as i32,
            slot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrestrial_ch_num() {
        let ch_str = "T13";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::Terrestrial(13));
        assert_eq!(ch.raw_string, ch_str.to_string());

        let ch_str = "T52";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::Terrestrial(52));
        assert_eq!(ch.raw_string, ch_str.to_string());

        let ch_str = "T53";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::Undefined);
        assert_eq!(ch.raw_string, ch_str.to_string());
    }
    fn test_bs_ch_num() {
        let ch_str = "BS1_3";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::Undefined);
        assert_eq!(ch.raw_string, ch_str.to_string());

        let ch_str = "BS13_3";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::BS(13, 3));
        assert_eq!(ch.raw_string, ch_str.to_string());
    }
    fn test_cs_ch_num() {
        let ch_str = "CS2";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::CS(2));
        assert_eq!(ch.raw_string, ch_str.to_string());

        let ch_str = "CS24";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::CS(2));
        assert_eq!(ch.raw_string, ch_str.to_string());

        let ch_str = "CS25";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(ch.ch_type, ChannelType::Undefined);
        assert_eq!(ch.raw_string, ch_str.to_string());
    }
    fn test_bon_chspace_from_str() {
        let ch_str = "1-2";
        let ch = Channel::from_ch_str(ch_str);
        assert_eq!(
            ch.ch_type,
            ChannelType::Bon(ChannelSpace {
                space: 1,
                ch: 2,
                space_description: None,
                ch_description: None
            })
        );
        assert_eq!(ch.raw_string, ch_str.to_string());
    }
}
