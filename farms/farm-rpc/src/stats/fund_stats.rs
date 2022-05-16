use {
    log::info,
    rusqlite::{Connection, OptionalExtension},
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_sdk::program_error::ProgramError,
};

#[allow(dead_code)]
pub const QUERY_LIMIT: u32 = 500;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Timeframe {
    Ticks,
    Hourly,
    Daily,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FundStatsRecord {
    pub timestamp: i64,
    pub assets_usd: f64,
    pub deposits_usd: f64,
    pub withdrawals_usd: f64,
}

pub struct FundStats {
    conn: Connection,
}

#[allow(dead_code)]
impl FundStats {
    pub fn new(db_path: &str) -> Result<Self, String> {
        info!("Opening database {}...", db_path);
        Ok(Self {
            conn: Connection::open(db_path).map_err(|e| e.to_string())?,
        })
    }

    pub fn update(
        &self,
        fund_name: &str,
        timestamp: i64,
        assets_usd: f64,
        deposits_usd: f64,
        withdrawals_usd: f64,
    ) -> Result<usize, String> {
        if !self.is_table_exists(fund_name)? {
            info!(
                "No existing tables found for the Fund \"{}\", creating new...",
                fund_name
            );
            self.init_table(fund_name)?;
            self.init_view(
                &(fund_name.to_string() + "_H"),
                fund_name,
                Timeframe::Hourly,
            )?;
            self.init_view(&(fund_name.to_string() + "_D"), fund_name, Timeframe::Daily)?;
        }

        self.conn.execute(
            &format!("REPLACE INTO '{}' (timestamp, assets_usd, deposits_usd, withdrawals_usd) values (?1, ?2, ?3, ?4)", fund_name),
            &[&timestamp.to_string(), &assets_usd.to_string(), &deposits_usd.to_string(), &withdrawals_usd.to_string()],
        ).map_err(|e| e.to_string())
    }

    pub fn select(
        &self,
        fund_name: &str,
        timeframe: Timeframe,
        start_time: i64,
        limit: u32,
    ) -> Result<Vec<FundStatsRecord>, String> {
        let table_name = fund_name.to_string()
            + match timeframe {
                Timeframe::Ticks => "",
                Timeframe::Hourly => "_H",
                Timeframe::Daily => "_D",
            };
        let limit = if limit == 0 {
            QUERY_LIMIT
        } else {
            std::cmp::min(limit, QUERY_LIMIT)
        };

        let mut query = if start_time > 0 {
            self.conn
                .prepare(&format!(
                    "SELECT * FROM '{}' WHERE timestamp >= {} LIMIT {}",
                    table_name, start_time, limit
                ))
                .map_err(|e| e.to_string())?
        } else {
            self.conn
                .prepare(&format!("SELECT * FROM '{}' LIMIT {}", table_name, limit))
                .map_err(|e| e.to_string())?
        };
        let res = query
            .query_map([], |row| {
                Ok(FundStatsRecord {
                    timestamp: row.get(0)?,
                    assets_usd: row.get(1)?,
                    deposits_usd: row.get(2)?,
                    withdrawals_usd: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|stat| stat.ok())
            .collect();

        Ok(res)
    }

    fn is_table_exists(&self, table_name: &str) -> Result<bool, String> {
        let res: Option<String> = self
            .conn
            .query_row(
                &format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                    table_name
                ),
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(row) = res {
            Ok(row == table_name)
        } else {
            Ok(false)
        }
    }

    fn init_table(&self, table_name: &str) -> Result<usize, String> {
        self.conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS '{}' (
                            timestamp integer primary key,
                            assets_usd real not null,
                            deposits_usd real not null,
                            withdrawals_usd real not null
                        )",
                    table_name
                ),
                [],
            )
            .map_err(|e| e.to_string())
    }

    fn init_view(
        &self,
        view_name: &str,
        source_table: &str,
        timeframe: Timeframe,
    ) -> Result<usize, String> {
        if matches!(timeframe, Timeframe::Ticks) {}
        let timeframe = match timeframe {
            Timeframe::Ticks => {
                return Err(format!("Invalid timeframe for the view {}", view_name));
            }
            Timeframe::Hourly => "%H",
            Timeframe::Daily => "%D",
        };
        self.conn
            .execute(
                &format!(
                    "CREATE VIEW IF NOT EXISTS '{}' as WITH windows AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY strftime('{}', timestamp, 'unixepoch') ORDER BY timestamp) idx FROM '{}') SELECT * FROM windows WHERE idx = 1;",
                    view_name, timeframe, source_table
                ),
                [],
            )
            .map_err(|e| e.to_string())
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Timeframe::Ticks => write!(f, "Ticks"),
            Timeframe::Hourly => write!(f, "Hourly"),
            Timeframe::Daily => write!(f, "Daily"),
        }
    }
}

impl std::str::FromStr for Timeframe {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "ticks" => Ok(Timeframe::Ticks),
            "hourly" => Ok(Timeframe::Hourly),
            "daily" => Ok(Timeframe::Daily),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl std::fmt::Display for FundStatsRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}
