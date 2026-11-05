use rusqlite::{params, Connection, Row};
use crate::error::SporeResult;
use crate::models::Track;
use tracing::debug;

pub struct CatalogStore {
    conn: Connection,
}

impl CatalogStore {
    pub fn open_in_memory() -> SporeResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_database(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_path(path: &str) -> SporeResult<Self> {
        let conn = Connection::open(path)?;
        Self::configure_database(&conn)?;
        Ok(Self { conn })
    }

    fn configure_database(conn: &Connection) -> SporeResult<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS tracks (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 artist TEXT NOT NULL,
                 album TEXT NOT NULL,
                 duration_ms INTEGER NOT NULL,
                 storefront TEXT NOT NULL,
                 preview_url TEXT,
                 artwork_url TEXT,
                 hls_manifest_url TEXT,
                 is_lossless INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
             CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);"
        )?;
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN artwork_url TEXT", []);
        Ok(())
    }

    pub fn delete_track(&self, id: &str) -> SporeResult<()> {
        self.conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_all_tracks(&self) -> SporeResult<()> {
        self.conn.execute("DELETE FROM tracks", [])?;
        Ok(())
    }

    
pub fn upsert_tracks(&self, tracks: &[Track]) -> SporeResult<()> { for t in tracks { self.upsert_track(t)?; } Ok(()) }
pub fn upsert_track(&self, track: &Track) -> SporeResult<()> {
        track.validate()?;
        self.conn.execute(
            "INSERT OR REPLACE INTO tracks (
                id, title, artist, album, duration_ms, storefront, preview_url, artwork_url, hls_manifest_url, is_lossless
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                track.id,
                track.title,
                track.artist,
                track.album,
                track.duration_ms as i64,
                track.storefront,
                &track.preview_url,
                &track.artwork_url,
                &track.hls_manifest_url,
                if track.is_lossless { 1 } else { 0 }
            ],
        )?;
        Ok(())
    }

    fn row_to_track(row: &Row<'_>) -> rusqlite::Result<Track> {
        let duration_i64: i64 = row.get(4)?;
        let lossless_i64: i64 = row.get(9)?;
        Ok(Track {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration_ms: duration_i64.max(0) as u64,
            storefront: row.get(5)?,
            preview_url: row.get::<_, Option<String>>(6)?,
            artwork_url: row.get::<_, Option<String>>(7)?,
            hls_manifest_url: row.get::<_, Option<String>>(8)?,
            is_lossless: lossless_i64 != 0,
        })
    }

    pub fn search_tracks(&self, query: &str, limit: usize) -> SporeResult<Vec<Track>> {
        let limit_i64 = limit as i64;
        if query.trim().is_empty() {
            debug!("Listing cached tracks (limit={})", limit);
            let mut stmt = self.conn.prepare(
                "SELECT id, title, artist, album, duration_ms, storefront, preview_url, artwork_url, hls_manifest_url, is_lossless
                 FROM tracks ORDER BY title ASC LIMIT ?1"
            )?;
            let rows = stmt.query_map(params![limit_i64], Self::row_to_track)?;
            let mut results = Vec::new();
            for item in rows {
                results.push(item?);
            }
            return Ok(results);
        }

        let pattern = format!("%{}%", query.trim());
        let mut stmt = self.conn.prepare(
            "SELECT id, title, artist, album, duration_ms, storefront, preview_url, artwork_url, hls_manifest_url, is_lossless
             FROM tracks
             WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1
             ORDER BY title ASC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![pattern, limit_i64], Self::row_to_track)?;
        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sqlite_store_upsert_and_search() {
        let store = CatalogStore::open_in_memory().expect("in-memory db");
        let track = Track {
            id: "100".to_string(),
            title: "Digital Love".to_string(),
            artist: "Daft Punk".to_string(),
            album: "Discovery".to_string(),
            duration_ms: 301_000,
            storefront: "cl".to_string(),
            preview_url: Some("https://audio-ssl.itunes.apple.com/test.m4a".to_string()),
            artwork_url: None,
            hls_manifest_url: None,
            is_lossless: true,
        };
        store.upsert_track(&track).expect("upsert");

        let found = store.search_tracks("daft", 10).expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Digital Love");
    }

    #[test]
    fn test_empty_query() {
        let store = CatalogStore::open_in_memory().expect("in-memory db");
        let tracks = vec![
            Track {
                id: "1".to_string(),
                title: "Song A".to_string(),
                artist: "Artist 1".to_string(),
                album: "Album 1".to_string(),
                duration_ms: 90_000,
                storefront: "storefront.com".to_string(),
                preview_url: None,
                artwork_url: None,
                hls_manifest_url: None,
                is_lossless: false,
            },
        ];
        for track in tracks {
            store.upsert_track(&track).unwrap();
        }

        let results = store.search_tracks("", 1).expect("empty query");
        assert_eq!(results.len(), 1);
    }
}
