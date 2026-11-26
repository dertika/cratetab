//! Dateien des Proxmox-LXC-Containers (Skripte und systemd-Units).
//!
//! Sie stecken in der Programmdatei, damit `cratetab-update` sie nach einem
//! Update auf den Stand der neuen Version bringen kann (`cratetab
//! install-lxc-tools`). Die Konfiguration `/etc/cratetab/cratetab.env` wird
//! dabei nie angefasst.

use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

/// (Ziel, Rechte, Inhalt)
const FILES: &[(&str, u32, &str)] = &[
    (
        "usr/local/bin/cratetab",
        0o755,
        include_str!("../packaging/lxc/cratetab"),
    ),
    (
        "usr/local/sbin/cratetab-backup",
        0o755,
        include_str!("../packaging/lxc/cratetab-backup"),
    ),
    (
        "usr/local/sbin/cratetab-update",
        0o755,
        include_str!("../packaging/lxc/cratetab-update"),
    ),
    (
        "etc/systemd/system/cratetab.service",
        0o644,
        include_str!("../packaging/lxc/cratetab.service"),
    ),
    (
        "etc/systemd/system/cratetab-backup.service",
        0o644,
        include_str!("../packaging/lxc/cratetab-backup.service"),
    ),
    (
        "etc/systemd/system/cratetab-backup.timer",
        0o644,
        include_str!("../packaging/lxc/cratetab-backup.timer"),
    ),
    (
        "etc/systemd/system/cratetab-update.service",
        0o644,
        include_str!("../packaging/lxc/cratetab-update.service"),
    ),
    (
        "etc/systemd/system/cratetab-update.timer",
        0o644,
        include_str!("../packaging/lxc/cratetab-update.timer"),
    ),
];

/// Verweise in /usr/bin und /usr/sbin: „pct exec <id> -- …“ übernimmt das PATH
/// der Proxmox-Shell, und das enthält nicht immer /usr/local/bin
const LINKS: &[(&str, &str)] = &[
    ("usr/bin/cratetab", "/usr/local/bin/cratetab"),
    (
        "usr/sbin/cratetab-backup",
        "/usr/local/sbin/cratetab-backup",
    ),
    (
        "usr/sbin/cratetab-update",
        "/usr/local/sbin/cratetab-update",
    ),
];

/// Schreibt Skripte, Units und Verweise unterhalb von `root` („/“ im Container).
/// Gibt die geänderten Dateien zurück.
pub fn install(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.join("etc/cratetab/cratetab.env").exists() {
        bail!(
            "{} fehlt – das ist kein Crate-Tab-LXC-Container",
            root.join("etc/cratetab/cratetab.env").display()
        );
    }
    let mut changed = Vec::new();
    for (rel, mode, content) in FILES {
        let target = root.join(rel);
        if fs::read(&target).is_ok_and(|old| old == content.as_bytes())
            && fs::metadata(&target).is_ok_and(|m| m.permissions().mode() & 0o7777 == *mode)
        {
            continue;
        }
        write_atomic(&target, content.as_bytes(), *mode)
            .with_context(|| format!("{} schreiben", target.display()))?;
        changed.push(target);
    }
    for (rel, dest) in LINKS {
        let link = root.join(rel);
        if fs::read_link(&link).is_ok_and(|d| d == Path::new(dest)) {
            continue;
        }
        let _ = fs::remove_file(&link);
        symlink(dest, &link).with_context(|| format!("{} anlegen", link.display()))?;
        changed.push(link);
    }
    Ok(changed)
}

/// Neue Datei daneben schreiben und umbenennen: Ein gerade laufendes Skript
/// (cratetab-update selbst) liest so ungestört die alte Fassung zu Ende.
fn write_atomic(target: &Path, content: &[u8], mode: u32) -> std::io::Result<()> {
    let dir = target.parent().expect("Pfad mit Verzeichnis");
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.new",
        target.file_name().expect("Dateiname").to_string_lossy()
    ));
    let mut f = fs::File::create(&tmp)?;
    f.write_all(content)?;
    f.set_permissions(fs::Permissions::from_mode(mode))?;
    f.sync_all()?;
    fs::rename(&tmp, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempRoot(PathBuf);
    impl TempRoot {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn installs_files_and_links_idempotently() {
        let dir = std::env::temp_dir().join(format!("cratetab-lxc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let root = TempRoot(dir);
        assert!(install(root.path()).is_err(), "ohne cratetab.env abbrechen");

        fs::create_dir_all(root.path().join("etc/cratetab")).unwrap();
        fs::write(root.path().join("etc/cratetab/cratetab.env"), "X=1\n").unwrap();
        for d in ["usr/bin", "usr/sbin"] {
            fs::create_dir_all(root.path().join(d)).unwrap();
        }
        // Alte Fassung eines Skripts wird ersetzt
        fs::create_dir_all(root.path().join("usr/local/sbin")).unwrap();
        fs::write(root.path().join("usr/local/sbin/cratetab-update"), "alt").unwrap();

        let changed = install(root.path()).unwrap();
        assert_eq!(changed.len(), FILES.len() + LINKS.len());
        let update = root.path().join("usr/local/sbin/cratetab-update");
        assert!(
            fs::read_to_string(&update)
                .unwrap()
                .contains("install-lxc-tools")
        );
        assert_eq!(
            fs::metadata(&update).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::read_link(root.path().join("usr/sbin/cratetab-update")).unwrap(),
            Path::new("/usr/local/sbin/cratetab-update")
        );
        // Konfiguration bleibt unberührt
        assert_eq!(
            fs::read_to_string(root.path().join("etc/cratetab/cratetab.env")).unwrap(),
            "X=1\n"
        );

        assert!(
            install(root.path()).unwrap().is_empty(),
            "zweiter Lauf ändert nichts"
        );
    }
}
