#![cfg(unix)]

mod support;

use std::{fs, path::PathBuf, time::Duration};

use support::{DAY, Fixture, Location, assert_absent, assert_present};

/// Ordinary roots: purpose x location (external or directly scanned) x whether
/// the link is also registered in `auto`. Structure only: angrr is not run, so
/// no cleanup expectations are implied.
#[test]
fn root_links_exist_at_every_location_and_can_be_registered() {
    for (location, register) in [
        (Location::External, false),
        (Location::External, true),
        (Location::GcRoots, false),
        (Location::GcRoots, true),
    ] {
        let fixture = Fixture::new();
        // Registrations nest under gcroots; the default config scans only this.
        assert_eq!(
            fixture.auto_directory(),
            fixture.directory(Location::GcRoots).join("auto")
        );
        let store = fixture.store_path("items/shared");
        let link = fixture.root_link(location, "nested/result", &store);

        assert_eq!(
            link.path.parent(),
            Some(fixture.directory(location).join("nested").as_path())
        );
        assert_eq!(fs::read_link(&link.path).unwrap(), store.path);
        assert_present(&link.path);
        assert_present(&store.path);

        let auto = fixture.auto_directory();
        if register {
            let registration = fixture.register_auto(&link);
            assert_eq!(registration.parent(), Some(auto.as_path()));
            assert_eq!(fs::read_link(&registration).unwrap(), link.path);
            assert_present(&registration);
        } else {
            assert!(fs::read_dir(&auto).unwrap().next().is_none());
        }
    }
}

/// Profile generations: location (external or profiles) x whether the
/// generation link is also registered in `auto`.
#[test]
fn profile_generations_exist_at_every_location_and_can_be_registered() {
    for (location, register) in [
        (Location::External, false),
        (Location::External, true),
        (Location::Profiles, false),
        (Location::Profiles, true),
    ] {
        let fixture = Fixture::new();
        let store = fixture.store_path("items/shared");
        let profile = fixture.profile(location, "nested/profile");
        let link = profile.generation(1, &store);

        assert_eq!(link.path, profile.generation_path(1));
        assert_eq!(
            link.path.parent(),
            Some(fixture.directory(location).join("nested").as_path())
        );
        assert_eq!(fs::read_link(&link.path).unwrap(), store.path);
        assert_present(&link.path);
        assert_present(&store.path);

        let auto = fixture.auto_directory();
        if register {
            let registration = fixture.register_auto(&link);
            assert_eq!(registration.parent(), Some(auto.as_path()));
            assert_eq!(fs::read_link(&registration).unwrap(), link.path);
            assert_present(&registration);
        } else {
            assert!(fs::read_dir(&auto).unwrap().next().is_none());
        }
    }
}

#[test]
fn registrations_are_unique_and_can_share_a_store_path() {
    let fixture = Fixture::new();
    let store = fixture.store_path("shared/item");
    let external = fixture.root_link(Location::External, "a", &store);
    let scanned = fixture.root_link(Location::GcRoots, "b", &store);
    let first = fixture.register_auto(&external);
    let second = fixture.register_auto(&external);
    let third = fixture.register_auto(&scanned);
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_ne!(first, third);
    for (registration, link) in [
        (&first, &external),
        (&second, &external),
        (&third, &scanned),
    ] {
        assert_eq!(fs::read_link(registration).unwrap(), link.path);
    }
    assert_eq!(fs::read_dir(fixture.auto_directory()).unwrap().count(), 3);
    assert_eq!(fs::read_dir(fixture.path.join("store")).unwrap().count(), 1);
    assert_eq!(fs::read_link(&external.path).unwrap(), store.path);
    assert_eq!(fs::read_link(&scanned.path).unwrap(), store.path);
}

#[test]
fn profile_handle_creates_generations_and_updates_the_current_link() {
    let fixture = Fixture::new();
    let profile = fixture.profile(Location::Profiles, "nested/profile");
    let first_path = fixture
        .directory(Location::Profiles)
        .join("nested/profile-1-link");
    assert_eq!(profile.generation_path(1), first_path);
    // The handle creates only the containing directory, not the current link.
    assert_absent(&profile.path);
    assert_absent(&first_path);
    assert_eq!(
        fs::read_dir(profile.path.parent().unwrap())
            .unwrap()
            .count(),
        0
    );

    let first = profile.generation(1, &fixture.store_path("gen-1"));
    assert_eq!(first.path, first_path);
    profile.set_current(1);
    assert_eq!(
        fs::read_link(&profile.path).unwrap(),
        PathBuf::from("profile-1-link")
    );

    let second = profile.generation(2, &fixture.store_path("gen-2"));
    profile.set_current(2);
    assert_eq!(
        fs::read_link(&profile.path).unwrap(),
        PathBuf::from("profile-2-link")
    );
    assert_present(&first.path);
    assert_present(&second.path);
}

#[test]
fn set_age_touches_only_the_link_mtime() {
    let fixture = Fixture::new();
    let store = fixture.store_path("aged");
    let link = fixture.root_link(Location::External, "aged", &store);
    let store_modified = fs::metadata(&store.path).unwrap().modified().unwrap();
    let link_before = fs::symlink_metadata(&link.path)
        .unwrap()
        .modified()
        .unwrap();

    link.set_age(Duration::from_secs(2 * DAY));

    let link_after = fs::symlink_metadata(&link.path)
        .unwrap()
        .modified()
        .unwrap();
    assert!(link_after < link_before);
    assert_eq!(fs::read_link(&link.path).unwrap(), store.path);
    assert_eq!(
        fs::metadata(&store.path).unwrap().modified().unwrap(),
        store_modified
    );
}
