//! UpgradeCustody instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            custody::{Custody, DeprecatedCustody, PositionStats, PricingParams},
            multisig::{AdminInstruction, Multisig},
            perpetuals::Perpetuals,
            pool::Pool,
        },
    },
    anchor_lang::prelude::*,
    solana_program::program_memory::sol_memcpy,
    std::{
        cmp,
        io::{self, Write},
    },
};

#[derive(Debug, Default)]
pub struct BpfWriter<T> {
    inner: T,
    pos: u64,
}

impl<T> BpfWriter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }
}

impl Write for BpfWriter<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.pos >= self.inner.len() as u64 {
            return Ok(0);
        }

        let amt = cmp::min(
            self.inner.len().saturating_sub(self.pos as usize),
            buf.len(),
        );
        sol_memcpy(&mut self.inner[(self.pos as usize)..], buf, amt);
        self.pos += amt as u64;
        Ok(amt)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.write(buf)? == buf.len() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to write whole buffer",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct UpgradeCustody<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"],
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(mut)]
    /// CHECK: Deprecated custody account
    pub custody: AccountInfo<'info>,

    system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpgradeCustodyParams {}

pub fn upgrade_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, UpgradeCustody<'info>>,
    params: &UpgradeCustodyParams,
) -> Result<u8> {
    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::UpgradeCustody, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // load deprecated custody data
    msg!("Load deprecated custody");
    let custody_account = &ctx.accounts.custody;
    if custody_account.owner != &crate::ID {
        return Err(ProgramError::IllegalOwner.into());
    }
    if custody_account.try_data_len()? != DeprecatedCustody::LEN {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let deprecated_custody = Account::<DeprecatedCustody>::try_from_unchecked(custody_account)?;

    let pricing = PricingParams {
        use_ema: deprecated_custody.pricing.use_ema,
        use_unrealized_pnl_in_aum: deprecated_custody.pricing.use_unrealized_pnl_in_aum,
        trade_spread_long: deprecated_custody.pricing.trade_spread_long,
        trade_spread_short: deprecated_custody.pricing.trade_spread_short,
        swap_spread: deprecated_custody.pricing.swap_spread,
        min_initial_leverage: deprecated_custody.pricing.min_initial_leverage,
        max_initial_leverage: deprecated_custody.pricing.max_leverage,
        max_leverage: deprecated_custody.pricing.max_leverage,
        max_payoff_mult: deprecated_custody.pricing.max_payoff_mult,
        max_utilization: 0,
        max_position_locked_usd: 0,
        max_total_locked_usd: 0,
    };

    // update custody data
    let custody_data = Custody {
        pool: deprecated_custody.pool,
        mint: deprecated_custody.mint,
        token_account: deprecated_custody.token_account,
        decimals: deprecated_custody.decimals,
        is_stable: deprecated_custody.is_stable,
        oracle: deprecated_custody.oracle,
        pricing,
        permissions: deprecated_custody.permissions,
        fees: deprecated_custody.fees,
        borrow_rate: deprecated_custody.borrow_rate,
        assets: deprecated_custody.assets,
        collected_fees: deprecated_custody.collected_fees,
        volume_stats: deprecated_custody.volume_stats,
        trade_stats: deprecated_custody.trade_stats,
        long_positions: PositionStats::default(),
        short_positions: PositionStats::default(),
        borrow_rate_state: deprecated_custody.borrow_rate_state,
        bump: deprecated_custody.bump,
        token_account_bump: deprecated_custody.token_account_bump,
    };

    if !custody_data.validate() {
        return err!(PerpetualsError::InvalidCustodyConfig);
    }

    msg!("Resize custody account");
    Perpetuals::realloc(
        ctx.accounts.admin.to_account_info(),
        ctx.accounts.custody.clone(),
        ctx.accounts.system_program.to_account_info(),
        Custody::LEN,
        true,
    )?;

    msg!("Re-initialize the custody");
    if custody_account.try_data_len()? != Custody::LEN {
        return Err(ProgramError::InvalidAccountData.into());
    }
    let mut data = custody_account.try_borrow_mut_data()?;
    let dst: &mut [u8] = &mut data;
    let mut writer = BpfWriter::new(dst);
    custody_data.try_serialize(&mut writer)?;

    Ok(0)
}
