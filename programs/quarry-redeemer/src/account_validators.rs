use crate::{CreateRedeemer, RedeemTokens};
use anchor_lang::prelude::*;
use vipers::prelude::*;

impl<'info> Validate<'info> for CreateRedeemer<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(
            self.iou_mint.decimals == self.redemption_mint.decimals,
            "decimals mismatch"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for RedeemTokens<'info> {
    fn validate(&self) -> Result<()> {
        invariant!(self.source_authority.is_signer, Unauthorized);

        assert_keys_eq!(self.iou_mint, self.redeemer.iou_mint);
        assert_keys_eq!(self.iou_source.mint, self.redeemer.iou_mint);
        assert_keys_eq!(self.iou_source.owner, self.source_authority);

        assert_keys_eq!(self.redemption_vault.owner, self.redeemer);
        assert_keys_eq!(self.redemption_vault.mint, self.redeemer.redemption_mint);
        invariant!(self.redemption_vault.delegate.is_none());
        invariant!(self.redemption_vault.close_authority.is_none());

        assert_keys_neq!(self.redemption_destination, self.redemption_vault);
        assert_keys_eq!(
            self.redemption_destination.mint,
            self.redeemer.redemption_mint
        );
        assert_keys_eq!(self.redemption_destination.owner, self.source_authority);

        Ok(())
    }
}
