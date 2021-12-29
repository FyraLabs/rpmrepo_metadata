use std::collections::HashMap;
// use rpm::{self, Header};
use std::convert::TryInto;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use quick_xml::Reader;

use crate::filelist::FilelistsXmlReader;
use crate::other::OtherXmlReader;
use crate::primary::PrimaryXmlReader;
use crate::{utils, RepoMdData};
use crate::{FilelistsXml, MetadataError, OtherXml, Package, PrimaryXml, EVR};

// impl TryInto<Package> for rpm::RPMPackage {
//     type Error = rpm::RPMError;

//     fn try_into(self) -> Result<Package, Self::Error> {
//         let mut pkg = Package::default();
//         pkg.set_name(self.metadata.header.get_name()?);
//         pkg.set_arch(self.metadata.header.get_arch()?);
//         {
//             let epoch = self.metadata.header.get_epoch()?.to_string(); // TODO evaluate epoch type
//             let version = self.metadata.header.get_version()?;
//             let release = self.metadata.header.get_release()?;
//             pkg.set_evr(EVR::new(epoch.as_str(), version, release));
//         }

//         //     checksum: todo!(),
//         //     location_href: todo!(),
//         //     summary: todo!(),
//         //     description: todo!(),
//         //     packager: todo!(),
//         //     url: todo!(),
//         //     time: todo!(),
//         //     size: todo!(),

//         //     rpm_license: todo!(),
//         //     rpm_vendor: todo!(),
//         //     rpm_group: todo!(),
//         //     rpm_buildhost: todo!(),
//         //     rpm_sourcerpm: todo!(),
//         //     rpm_header_range: todo!(),

//         //     rpm_requires: todo!(),
//         //     rpm_provides: todo!(),
//         //     rpm_conflicts: todo!(),
//         //     rpm_obsoletes: todo!(),
//         //     rpm_suggests: todo!(),
//         //     rpm_enhances: todo!(),
//         //     rpm_recommends: todo!(),
//         //     rpm_supplements: todo!(),

//         //     rpm_changelogs: todo!(),
//         //     rpm_files: todo!(),
//         // };

//         Ok(pkg)
//     }
// }

pub struct PackageParser {
    primary_xml: PrimaryXmlReader<BufReader<Box<dyn std::io::Read>>>,
    filelists_xml: FilelistsXmlReader<BufReader<Box<dyn std::io::Read>>>,
    other_xml: OtherXmlReader<BufReader<Box<dyn std::io::Read>>>,

    num_packages: usize,
    num_remaining: usize,
    in_progress_package: Option<Package>,
    // unfinished_packages: HashMap<String, Package>,
}

impl PackageParser {
    pub fn from_files(
        primary_path: &Path,
        filelists_path: &Path,
        other_path: &Path,
    ) -> Result<Self, MetadataError> {
        let primary_xml = PrimaryXml::new_reader(utils::xml_reader_from_path(primary_path)?);
        let filelists_xml = FilelistsXml::new_reader(utils::xml_reader_from_path(filelists_path)?);
        let other_xml = OtherXml::new_reader(utils::xml_reader_from_path(other_path)?);

        Self::from_readers(primary_xml, filelists_xml, other_xml)
    }

    pub fn from_readers(
        primary_xml: PrimaryXmlReader<BufReader<Box<dyn std::io::Read>>>,
        filelists_xml: FilelistsXmlReader<BufReader<Box<dyn std::io::Read>>>,
        other_xml: OtherXmlReader<BufReader<Box<dyn std::io::Read>>>,
    ) -> Result<Self, MetadataError> {
        let mut parser = PackageParser {
            primary_xml,
            filelists_xml,
            other_xml,
            num_packages: 0,
            num_remaining: 0,
            in_progress_package: None,
            // unfinished_packages: HashMap::new(),
        };
        parser.parse_headers()?;

        Ok(parser)
    }

    fn parse_headers(&mut self) -> Result<(), MetadataError> {
        let primary_pkg_count = self.primary_xml.read_header()?;
        let filelists_pkg_count = self.filelists_xml.read_header()?;
        let other_pkg_count = self.other_xml.read_header()?;

        assert_eq!(primary_pkg_count, filelists_pkg_count);
        assert_eq!(primary_pkg_count, other_pkg_count);
        self.num_packages = primary_pkg_count;
        self.num_remaining = self.num_packages;

        Ok(())
    }

    pub fn parse_package(&mut self) -> Result<Option<Package>, MetadataError> {
        self.primary_xml
            .read_package(&mut self.in_progress_package)?;
        self.filelists_xml
            .read_package(&mut self.in_progress_package)?;
        self.other_xml.read_package(&mut self.in_progress_package)?;

        let package = self.in_progress_package.take();

        // TODO: error handling?
        if let Some(_) = package {
            self.num_remaining = self
                .num_remaining
                .checked_sub(1)
                .expect("More packages parsed than declared in the metadata header.");
        } else {
            assert!(
                self.num_remaining == 0,
                "Less packages parsed than declared in metadata header."
            );
        }

        Ok(package)
    }

    pub fn remaining_packages(&self) -> usize {
        self.num_remaining
    }

    pub fn total_packages(&self) -> usize {
        self.num_packages
    }
}

impl Iterator for PackageParser {
    type Item = Result<Package, MetadataError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.parse_package().transpose()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.remaining_packages()))
    }
}
