use {
    crate::fund_stats::{FundStats, FundStatsRecord},
    log::{debug, info},
    solana_farm_client::client::FarmClient,
    std::{collections::HashMap, thread, time::Duration},
};

pub fn collect(
    farm_client_url: &str,
    sqlite_db_path: &str,
    update_interval_sec: u64,
) -> Result<(), String> {
    let db = FundStats::new(sqlite_db_path)?;
    let client = FarmClient::new(farm_client_url);
    let mut last_updates: HashMap<String, i64> = HashMap::new();

    loop {
        let funds = client.get_funds().map_err(|e| e.to_string())?;

        for fund_name in funds.keys() {
            let fund_stats = client.get_fund_info(fund_name).map_err(|e| e.to_string())?;
            let last_update = *last_updates.get(fund_name).unwrap_or(&0);
            if fund_stats.assets_update_time > 0 && last_update != fund_stats.assets_update_time {
                debug!(
                    "Updating Fund \"{}\" with {}...",
                    fund_name,
                    FundStatsRecord {
                        timestamp: fund_stats.assets_update_time,
                        assets_usd: fund_stats.current_assets_usd,
                        deposits_usd: fund_stats.amount_invested_usd,
                        withdrawals_usd: fund_stats.amount_removed_usd,
                    }
                );
                db.update(
                    fund_name,
                    fund_stats.assets_update_time,
                    fund_stats.current_assets_usd,
                    fund_stats.amount_invested_usd,
                    fund_stats.amount_removed_usd,
                )?;
                last_updates.insert(fund_name.clone(), last_update);
            }
        }

        if update_interval_sec > 1 {
            info!(
                "Update complete, next check in {} secs...",
                update_interval_sec
            );
        }
        thread::sleep(Duration::from_secs(update_interval_sec));
    }
}
