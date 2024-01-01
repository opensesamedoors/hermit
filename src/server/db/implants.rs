use log::warn;
use rusqlite::{Connection, Result};

use crate::server::implants::implant::Implant;

pub fn init_implants(db_path: String) -> Result<()> {
    let db = match Connection::open(db_path) {
        Ok(d) => d,
        Err(e) => { 
            return Err(e);
        }
    };

    db.execute(
        "CREATE TABLE implants (
            id              INTEGER PRIMARY KEY,
            name            TEXT NOT NULL,
            listener_url    TEXT NOT NULL,
            os              TEXT NOT NULL,
            arch            TEXT NOT NULL,
            format          TEXT NOT NULL,
            sleep           INTEGER NOT NULL
        )",
        ()
    )?;

    Ok(())
}

pub fn add_implant(db_path: String, implant: Implant) -> Result<()> {
    let db = match Connection::open(db_path.to_owned()) {
        Ok(d) => d,
        Err(e) => { 
            return Err(e);
        }
    };

    let exists = exists_implant(
        db_path.to_owned(),
        implant.clone()
    )?;

    if exists {
        warn!("Implant already exists.");
        return Ok(())
    }

    db.execute(
        "INSERT INTO implants (name, listener_url, os, arch, format, sleep) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            implant.name,
            implant.listener_url,
            implant.os,
            implant.arch,
            implant.format,
            implant.sleep,
        )
    )?;

    Ok(())
}

pub fn exists_implant(db_path: String, implant: Implant) -> Result<bool> {
    let db = match Connection::open(db_path) {
        Ok(d) => d,
        Err(e) => { 
            return Err(e);
        }
    };

    let mut stmt = db.prepare(
        "SELECT * FROM implants WHERE listener_url = ?1 AND os = ?2 AND arch = ?3 AND format = ?4 AND sleep = ?5"
    )?;
    let exists = stmt.exists(
        [
            implant.listener_url,
            implant.os,
            implant.arch,
            implant.format,
            implant.sleep.to_string(),
        ]
    )?;

    Ok(exists)
}

pub fn delete_implant(db_path: String, implant_name: String) -> Result<()> {
    let db = match Connection::open(db_path) {
        Ok(d) => d,
        Err(e) => { 
            return Err(e);
        }
    };

    db.execute(
        "DELETE FROM implants WHERE id = ?1 OR name = ?2",
        [implant_name.to_string(), implant_name.to_string()],
    )?;
    
    Ok(())
}

pub fn get_all_implants(db_path: String) -> Result<Vec<Implant>> {
    let mut implants: Vec<Implant> = Vec::new();

    let db = match Connection::open(db_path) {
        Ok(d) => d,
        Err(e) => { 
            return Err(e);
        }
    };

    let mut stmt = db.prepare(
        "SELECT id, name, listener_url, os, arch, format, sleep FROM implants"
    )?;
    let implant_iter = stmt.query_map([], |row| {
        Ok(Implant::new(
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?;

    for implant in implant_iter {
        implants.push(implant.unwrap());
    }

    Ok(implants)
}