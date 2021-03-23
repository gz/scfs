use std::convert::TryInto;
use std::path::{Path, PathBuf};

use debpkg::{Control, DebPkg};
use indicatif::ParallelProgressIterator;
use log::error;
use rayon::prelude::*;
use walkdir::WalkDir;

use scfs_ddlog::typedefs::ddlog_std;
use scfs_ddlog::typedefs::{Comparator, Dependency, Package};

fn deb_str_to_comparator(s: &str) -> Comparator {
    match s {
        "<<" => Comparator::StrictlyEarlier,
        "<=" => Comparator::EarlierOrEqual,
        "=" => Comparator::ExactlyEqual,
        ">=" => Comparator::LaterOrEqual,
        ">>" => Comparator::StrictlyLater,
        x => panic!(
            "Unknown comparator {}. Update `deb_str_to_comparator` and scfs.dl!",
            x
        ),
    }
}

/// Potential tags that can appear in our .deb packets
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
enum Tags {
    Package,
    Version,
    Source,
    Architecture,
    Maintainer,
    OriginalMaintainer,
    InstalledSize,
    Replaces,
    Section,
    MultiArch,
    Homepage,
    Description,
    Breaks,
    Depends,
    Suggests,
    Priority,
    BuiltUsing,
    Recommends,
    Conflicts,
    Provides,
    Enhances,
    BuildIds,
    PreDepends,
    Tag,
    Essential,
    Bugs,
    Task,
    Important,
    Modaliases,
    CnfVisiblePkgname,
    CnfExtraCommands,
    CnfIgnoreCommands,
    UbuntuOemKernelFlavour,
    RubyVersions,
    LuaVersions,
    PythonVersion,
    Python3Version,
    PythonEggName,
    XCargoBuiltUsing,
    GhcPackage,
    GoImportPath,
    GstreamerElements,
    GstreamerEncoders,
    GstreamerDecoders,
    GstreamerVersion,
    GstreamerUriSources,
    GstreamerUriSinks,
    OriginalVcsGit,
    EfiVendor,
    OriginalVcsBrowser,
    PostgresqlCatversion,
    XulAppid,
    NppApplications,
    NppDescription,
    NppFile,
    NppMimetype,
    Unknown(String),
}

impl Tags {
    fn field_name(&self) -> &str {
        match self {
            Tags::Package => "Package",
            Tags::Version => "Version",
            Tags::Source => "Source",
            Tags::Architecture => "Architecture",
            Tags::Maintainer => "Maintainer",
            Tags::OriginalMaintainer => "Original-Maintainer",
            Tags::InstalledSize => "Installed-Size",
            Tags::Replaces => "Replaces",
            Tags::Section => "Section",
            Tags::MultiArch => "Multi-Arch",
            Tags::Homepage => "Homepage",
            Tags::Description => "Description",
            Tags::Breaks => "Breaks",
            Tags::Depends => "Depends",
            Tags::Suggests => "Suggests",
            Tags::Priority => "Priority",
            Tags::BuiltUsing => "Built-Using",
            Tags::Recommends => "Recommends",
            Tags::Conflicts => "Conflicts",
            Tags::Provides => "Provides",
            Tags::Enhances => "Enhances",
            Tags::BuildIds => "Build-Ids",
            Tags::PreDepends => "Pre-Depends",
            Tags::Essential => "Essential",
            Tags::Bugs => "Bugs",
            Tags::Tag => "Tag",
            Tags::UbuntuOemKernelFlavour => "Ubuntu-Oem-Kernel-Flavour",
            Tags::OriginalVcsGit => "Original-Vcs-Git",
            Tags::RubyVersions => "Ruby-Versions",
            Tags::LuaVersions => "Lua-Versions",
            Tags::PythonVersion => "Python-Version",
            Tags::PythonEggName => "Python-Egg-Name",
            Tags::GhcPackage => "Ghc-Package",
            Tags::XCargoBuiltUsing => "X-Cargo-Built-Using",
            Tags::CnfVisiblePkgname => "Cnf-Visible-Pkgname",
            Tags::CnfIgnoreCommands => "Cnf-Ignore-Commands",
            Tags::CnfExtraCommands => "Cnf-Extra-Commands",
            Tags::GoImportPath => "Go-Import-Path",
            Tags::GstreamerElements => "Gstreamer-Elements",
            Tags::GstreamerDecoders => "Gstreamer-Decoders",
            Tags::GstreamerEncoders => "Gstreamer-Encoders",
            Tags::GstreamerVersion => "Gstreamer-Version",
            Tags::GstreamerUriSources => "Gstreamer-Uri-Sources",
            Tags::GstreamerUriSinks => "Gstreamer-Uri-Sinks",
            Tags::Python3Version => "Python3-Version",
            Tags::EfiVendor => "Efi-Vendor",
            Tags::OriginalVcsBrowser => "Original-Vcs-Browser",
            Tags::Modaliases => "Modaliases",
            Tags::PostgresqlCatversion => "Postgresql-Catversion",
            Tags::XulAppid => "Xul-Appid",
            Tags::Task => "Task",
            Tags::Important => "Important",
            Tags::NppApplications => "Npp-Applications",
            Tags::NppDescription => "Npp-Description",
            Tags::NppFile => "Npp-File",
            Tags::NppMimetype => "Npp-Mimetype",
            Tags::Unknown(x) => x.as_str(),
        }
    }
}

impl From<&str> for Tags {
    fn from(tag: &str) -> Self {
        match tag {
            "Package" => Tags::Package,
            "Version" => Tags::Version,
            "Source" => Tags::Source,
            "Architecture" => Tags::Architecture,
            "Maintainer" => Tags::Maintainer,
            "Original-Maintainer" => Tags::OriginalMaintainer,
            "Installed-Size" => Tags::InstalledSize,
            "Replaces" => Tags::Replaces,
            "Section" => Tags::Section,
            "Multi-Arch" => Tags::MultiArch,
            "Homepage" => Tags::Homepage,
            "Description" => Tags::Description,
            "Breaks" => Tags::Breaks,
            "Depends" => Tags::Depends,
            "Suggests" => Tags::Suggests,
            "Priority" => Tags::Priority,
            "Built-Using" => Tags::BuiltUsing,
            "Recommends" => Tags::Recommends,
            "Conflicts" => Tags::Conflicts,
            "Provides" => Tags::Provides,
            "Enhances" => Tags::Enhances,
            "Build-Ids" => Tags::BuildIds,
            "Pre-Depends" => Tags::PreDepends,
            "Essential" => Tags::Essential,
            "Bugs" => Tags::Bugs,
            "Tag" => Tags::Tag,
            "Ubuntu-Oem-Kernel-Flavour" => Tags::UbuntuOemKernelFlavour,
            "Original-Vcs-Git" => Tags::OriginalVcsGit,
            "Ruby-Versions" => Tags::RubyVersions,
            "Lua-Versions" => Tags::LuaVersions,
            "Python-Version" => Tags::PythonVersion,
            "Python-Egg-Name" => Tags::PythonEggName,
            "Ghc-Package" => Tags::GhcPackage,
            "X-Cargo-Built-Using" => Tags::XCargoBuiltUsing,
            "Cnf-Visible-Pkgname" => Tags::CnfVisiblePkgname,
            "Cnf-Ignore-Commands" => Tags::CnfIgnoreCommands,
            "Cnf-Extra-Commands" => Tags::CnfExtraCommands,
            "Go-Import-Path" => Tags::GoImportPath,
            "Gstreamer-Elements" => Tags::GstreamerElements,
            "Gstreamer-Decoders" => Tags::GstreamerDecoders,
            "Gstreamer-Encoders" => Tags::GstreamerEncoders,
            "Gstreamer-Version" => Tags::GstreamerVersion,
            "Gstreamer-Uri-Sources" => Tags::GstreamerUriSources,
            "Gstreamer-Uri-Sinks" => Tags::GstreamerUriSinks,
            "Python3-Version" => Tags::Python3Version,
            "Efi-Vendor" => Tags::EfiVendor,
            "Original-Vcs-Browser" => Tags::OriginalVcsBrowser,
            "Modaliases" => Tags::Modaliases,
            "Postgresql-Catversion" => Tags::PostgresqlCatversion,
            "Xul-Appid" => Tags::XulAppid,
            "Task" => Tags::Task,
            "Important" => Tags::Important,
            "Npp-Applications" => Tags::NppApplications,
            "Npp-Description" => Tags::NppDescription,
            "Npp-File" => Tags::NppFile,
            "Npp-Mimetype" => Tags::NppMimetype,
            _ => {
                error!(
                    "Unknown tag: '{}' (You may want to update the Tags enum).",
                    tag
                );
                Tags::Unknown(tag.into())
            }
        }
    }
}

pub fn parse_package<P: AsRef<Path>>(path: &P) -> Package {
    let file = std::fs::File::open(path).unwrap();
    let mut pkg = DebPkg::parse(file).unwrap();
    let control_tar = pkg.control().unwrap();
    let control = Control::extract(control_tar).unwrap(); // This can fail :O

    let mut data_tar = pkg.data().unwrap();
    let mut file_paths: Vec<String> = Vec::new();
    for file in data_tar.entries().unwrap() {
        let file = file.unwrap();
        let path = file.path().unwrap();
        file_paths.push(path.to_path_buf().display().to_string());
    }

    let mut p: Package = Default::default();
    p.package = control.name().to_string();
    p.version = control.version().to_string();
    p.files = file_paths.into();

    for tag in control.tags() {
        let tag: Tags = tag.into();
        match tag {
            Tags::Source => p.source = control.get(tag.field_name()).map(|t| t.to_string()).into(),
            Tags::Architecture => {
                p.architecture = control.get(tag.field_name()).map(|t| t.to_string()).into()
            }
            Tags::Maintainer => {
                p.maintainer = control.get(tag.field_name()).map(|t| t.to_string()).into()
            }
            Tags::OriginalMaintainer => {
                p.original_maintainer = control.get(tag.field_name()).map(|t| t.to_string()).into()
            }
            Tags::Depends => {
                // Parses a string like this: "libc6 (>= 2.29), libqt5gui5 (>=
                // 5.5) | libqt5gui5-gles (>= 5.5)" (e.g., requires libc6 AND
                // (libqt5gui5 OR libqt5gui5-gles))
                //
                // First each dependency is split by `,` for the ANDs, then
                // split by `|` for the ORs. The ORs just extend the Vec<>
                // fields within a single Dependency.
                //
                // More about version constraints:
                // https://www.debian.org/doc/debian-policy/ch-controlfields.html#version
                // https://www.debian.org/doc/debian-policy/ch-relationships.html
                p.depends = ddlog_std::Vec::new();
                let dependencies_line = control.get(tag.field_name()).unwrap_or("");

                for or_dependency in dependencies_line.split(',').collect::<Vec<&str>>() {
                    let mut d: Dependency = Default::default();

                    for dependency in or_dependency.split("|").collect::<Vec<&str>>() {
                        match dependency.rfind("(") {
                            Some(mid) => {
                                let (name, version_line) = dependency.split_at(mid);
                                let name = name.trim(); // Skip space
                                let version_line = version_line
                                    .trim_start_matches('(')
                                    .trim_end_matches(')')
                                    .trim();

                                match version_line.rfind(' ') {
                                    Some(mid) => {
                                        let (vconstraint, version) = version_line.split_at(mid);
                                        d.package.push(name.to_string());
                                        d.version.push(
                                            Some(
                                                (
                                                    deb_str_to_comparator(vconstraint.trim()),
                                                    version.trim().to_string(),
                                                )
                                                    .into(),
                                            )
                                            .into(),
                                        );
                                    }
                                    None => {
                                        unreachable!(
                                        "We should find some version constraint (==, >= etc.) in: {}",
                                        version_line
                                    );
                                    }
                                }
                            }
                            None => {
                                d.package.push(dependency.trim().to_string());
                                // No version constraint
                                d.version.push(None.into());
                            }
                        }
                    }

                    p.depends.push(d);
                }
            }
            Tags::Replaces => {
                p.replaces = control.get(tag.field_name()).map(|t| t.to_string()).into();
            }
            Tags::Section => {
                p.section = control.get(tag.field_name()).map(|t| t.to_string()).into();
            }
            Tags::MultiArch => {
                p.multi_arch = control.get(tag.field_name()).map(|t| t.to_string()).into()
            }
            Tags::Homepage => {
                p.homepage = control.get(tag.field_name()).map(|t| t.to_string()).into()
            }
            Tags::Description => {
                p.description = control.long_description().map(|t| t.to_string()).into()
            }
            _ => { /* Ignore all with other fields */ }
        }
    }

    p
}

pub fn parse_packages(root: PathBuf) -> Result<ddlog_std::Vec<Package>, String> {
    let mut deb_paths = Vec::with_capacity(1024 * 1024);

    for entry in WalkDir::new(root) {
        let entry = entry.unwrap();
        let path = entry.path();
        let extension = path.extension();

        let is_deb_file = extension.map_or(false, |ext| ext == "deb");
        if is_deb_file {
            deb_paths.push(path.to_path_buf());
        }
    }

    let packages: Vec<_> = deb_paths
        .par_iter()
        .progress_count(deb_paths.len().try_into().unwrap())
        .map(|path| parse_package(path))
        .collect();

    Ok(packages.into())
}
