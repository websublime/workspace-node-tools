use std::fmt::{ Display, Formatter, Error };

#[derive(Debug)]
pub enum PackageJsonErrorEnum {
    InvalidName(String),
    InvalidDescription(String),
    InvalidVersion(String),
    UnknownErr,
}

impl std::error::Error for PackageJsonErrorEnum {}

impl Display for PackageJsonErrorEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PackageJsonErrorEnum::InvalidName(message) => {
                write!(f, "{}", message)
            }
            _ => todo!(),
        }
    }
}

// Err(PackageJsonErrorEnum::InvalidName("No valid name"))
