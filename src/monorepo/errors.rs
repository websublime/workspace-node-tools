use std::fmt::{Display, Error, Formatter};

#[derive(Debug)]
pub enum PackageJsonErrorEnum {
    InvalidName(String),
    InvalidDescription(String),
    InvalidVersion(String),
    InvalidRepository(String),
    InvalidFiles(String),
    InvalidLicense(String),
    UnknownErr,
}

impl std::error::Error for PackageJsonErrorEnum {}

impl Display for PackageJsonErrorEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PackageJsonErrorEnum::InvalidName(message) => {
                write!(f, "{}", message)
            }
            PackageJsonErrorEnum::InvalidDescription(message) => {
                write!(f, "{}", message)
            }
            PackageJsonErrorEnum::InvalidVersion(message) => {
                write!(f, "{}", message)
            }
            PackageJsonErrorEnum::InvalidRepository(message) => {
                write!(f, "{}", message)
            }
            PackageJsonErrorEnum::InvalidFiles(message) => {
                write!(f, "{}", message)
            }
            PackageJsonErrorEnum::InvalidLicense(message) => {
                write!(f, "{}", message)
            }
            _ => todo!(),
        }
    }
}

// Err(PackageJsonErrorEnum::InvalidName("No valid name"))
