use crate::{CreateRedeemer, RedeemTokens};
use anchor_lang::prelude::*;
use vipers::validate::Validate;
use vipers::{assert_ata, assert_keys, invariant};

impl<'info> Validate<'info> for CreateRedeemer<'info> {
    fn validate(&self) -> ProgramResult {
        invariant!(
            self.iou_mint.decimals == self.redemption_mint.decimals,
            "decimals mismatch"
        );
        Ok(())
    }
}

impl<'info> Validate<'info> for RedeemTokens<'info> {
    fn validate(&self) -> ProgramResult {
        require!(self.source_authority.is_signer, Unauthorized);
        assert_keys!(self.iou_mint, self.redeemer.iou_mint, "iou_mint");
        assert_keys!(
            self.iou_source.mint,
            self.redeemer.iou_mint,
            "iou_source.mint"
        );
        assert_keys!(
            self.iou_source.owner,
            self.source_authority,
            "iou_source.owner"
        );

        assert_ata!(
            self.redemption_vault,
            self.redeemer,
            self.redeemer.redemption_mint,
            "redemption_vault"
        );
        assert_keys!(
            self.redemption_destination.mint,
            self.redeemer.redemption_mint,
            "redemption_destination.mint"
        );
        assert_keys!(
            self.redemption_destination.owner,
            self.source_authority,
            "redemption_destination.owner"
        );

        Ok(())
    }
}
