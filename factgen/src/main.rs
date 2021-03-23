use std::borrow::Cow;
use std::path::PathBuf;

use scfs_ddlog::api::HDDlog;
use scfs_ddlog::relid2name;
use scfs_ddlog::typedefs::ddlog_std::{Option, Ref};
use scfs_ddlog::typedefs::*;
use scfs_ddlog::Relations;

use differential_datalog::DeltaMap;
use differential_datalog::{DDlog, DDlogDynamic, DDlogInventory}; // Trait that must be implemented by an instance of a DDlog program. // Type that represents a set of changes to DDlog relations.
                                                                 // Returned by `DDlog::transaction_commit_dump_changes()`.
use differential_datalog::ddval::DDValConvert;
use differential_datalog::ddval::DDValue; // Generic type that wraps all DDlog value. // Trait to convert Rust types to/from DDValue.
                                          // All types used in input and output relations, indexes, and
                                          // primary keys implement this trait.
use differential_datalog::program::RelId; // Numeric relations id.
use differential_datalog::program::Update; // Type-safe representation of a DDlog command (insert/delete_val/delete_key/...)

// The `record` module defines dynamically typed representation of DDlog values and commands.
use differential_datalog::record::Record; // Dynamically typed representation of DDlog values.
use differential_datalog::record::RelIdentifier; // Relation identifier: either `RelId` or `Cow<str>`g.
use differential_datalog::record::UpdCmd; // Dynamically typed representation of DDlog command.
use differential_datalog::DDlogDump;

mod parse;

fn main() -> Result<(), String> {
    env_logger::init();

    let mut root = PathBuf::new();
    root.push("..");
    root.push("apt-mirror/mirror/archive.ubuntu.com/ubuntu/pool/main/a");
    let packages = parse::parse_packages(root)?;

    let threads = 1;
    let debug = false;
    let (mut hddlog, init_state) = HDDlog::run(threads, debug)?;

    println!("Initial state");
    dump_delta(&init_state);

    hddlog.transaction_start()?;
    let mut updates = Vec::with_capacity(packages.len());
    for p in packages {
        updates.push(Update::Insert {
            relid: Relations::Package as RelId,
            v: Ref::from(p).into_ddvalue(),
        });
    }
    hddlog.apply_updates(&mut updates.into_iter())?;
    let mut delta = hddlog.transaction_commit_dump_changes()?;

    println!("\nState after transaction 1");
    dump_delta(&delta);

    println!("\nEnumerating new packages");
    // Retrieve the set of changes for a particular relation.
    let new_packages = delta.get_rel(Relations::Package as RelId);
    for (val, weight) in new_packages.iter() {
        assert_eq!(*weight, 1);
        let package: &Package = unsafe { Package::from_ddvalue_ref(val) };
        println!("New package: {}", package.package);
    }

    let cback: fn(&Record, isize) -> bool = |record, sz| {
        println!("sz = {}", sz);
        true
    };

    println!("ddlog dump");
    hddlog.dump_table(Relations::Package as RelId, Some(&cback));
    println!("ddlog stop");

    hddlog.stop().unwrap();
    Ok(())
}

fn dump_delta(delta: &DeltaMap<DDValue>) {
    for (rel, changes) in delta.iter() {
        println!("Changes to relation {}", relid2name(*rel).unwrap());
        for (val, weight) in changes.iter() {
            println!("{} {:+}", val, weight);
        }
    }
}
