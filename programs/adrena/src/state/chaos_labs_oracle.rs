use {
    crate::error::AdrenaError,
    anchor_lang::prelude::*,
    solana_program::{keccak::hashv, secp256k1_recover},
};

// To be used in tests for fake data, we do use a dummy keypair to sign the prices that match this pubkey
#[cfg(feature = "test")]
pub const CHAOS_LABS_ORACLE_SIGNER_PUBKEY: [u8; 64] = [
    77, 166, 61, 132, 190, 106, 186, 61, 124, 242, 110, 254, 16, 241, 75, 156, 168, 27, 197, 240,
    46, 234, 45, 63, 19, 93, 241, 5, 186, 26, 52, 224, 56, 57, 212, 10, 56, 209, 53, 162, 228, 43,
    56, 81, 177, 132, 255, 115, 81, 136, 223, 47, 215, 56, 85, 1, 176, 44, 226, 76, 168, 73, 246,
    216,
];

// To be used in production for real data
#[cfg(not(feature = "test"))]
pub const CHAOS_LABS_ORACLE_SIGNER_PUBKEY: [u8; 64] = [
    226, 136, 204, 246, 125, 168, 38, 230, 184, 157, 110, 105, 39, 190, 158, 58, 61, 8, 224, 25,
    89, 45, 242, 179, 235, 181, 118, 99, 112, 253, 92, 46, 143, 183, 26, 181, 41, 16, 29, 111, 43,
    44, 168, 42, 240, 165, 48, 8, 182, 246, 94, 59, 32, 196, 135, 194, 246, 132, 210, 41, 178, 75,
    112, 61,
];

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ChaosLabsBatchPrices {
    pub prices: Vec<PriceData>,
    pub signature: [u8; 64],
    pub recovery_id: u8,
}

/// Individual price data within a batch
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Copy)]
pub struct PriceData {
    pub feed_id: u8,
    pub price: u64,
    pub timestamp: i64,
}

impl ChaosLabsBatchPrices {
    /// Build a message hash from price data entries
    /// This creates a deterministic hash that matches what was signed by Edge
    pub fn build_message_hash(&self) -> Result<[u8; 32]> {
        // Create buffers for each price entry
        let mut buffers = Vec::with_capacity(self.prices.len());

        for price in &self.prices {
            // Format each price entry:
            // 1. feed_id (enum) (1 byte)
            // 2. price (8 bytes, little-endian)
            // 3. expo (1 byte)
            // 4. timestamp (8 bytes, little-endian)

            let mut msg = vec![0u8; 32];

            // Append feed_id as a single byte
            msg[0] = price.feed_id;

            // Append price as 8 bytes little-endian
            msg.extend_from_slice(&price.price.to_le_bytes());

            // Append expo as a single byte
            msg.push(-10_i8 as u8);

            // Append timestamp as 8 bytes little-endian
            msg.extend_from_slice(&price.timestamp.to_le_bytes());

            buffers.push(msg);
        }

        // Use Solana's hashv function to hash all price entries together
        let buffer_refs: Vec<&[u8]> = buffers.iter().map(|buf| buf.as_slice()).collect();
        let hash = hashv(&buffer_refs);

        Ok(hash.to_bytes())
    }

    pub fn verify_signature(&self) -> Result<()> {
        self.verify_signature_with_pubkey(&CHAOS_LABS_ORACLE_SIGNER_PUBKEY)
    }

    /// Verifies a batch of prices with a single signature
    pub fn verify_signature_with_pubkey(
        &self,
        expected_signer_uncompressed_pubkey: &[u8; 64],
    ) -> Result<()> {
        let message_hash = self.build_message_hash()?;

        // Use Solana's secp256k1_recover function to recover the public key from the signature
        let recovered_pubkey =
            secp256k1_recover::secp256k1_recover(&message_hash, self.recovery_id, &self.signature)
                .map_err(|e| match e {
                    secp256k1_recover::Secp256k1RecoverError::InvalidRecoveryId => {
                        msg!("Secp256k1RecoverError: Invalid recovery ID");
                        AdrenaError::InvalidOracleSignature
                    }
                    secp256k1_recover::Secp256k1RecoverError::InvalidSignature => {
                        msg!("Secp256k1RecoverError: Invalid signature");
                        AdrenaError::InvalidOracleSignature
                    }
                    _ => {
                        msg!("Secp256k1RecoverError: Unknown error");
                        AdrenaError::InvalidOracleSignature
                    }
                })?;

        // Compare serialized keys
        require!(
            recovered_pubkey.to_bytes() == *expected_signer_uncompressed_pubkey,
            AdrenaError::InvalidOracleSignature,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, hex::FromHex};

    // Testing the signature verification with staging chaos labs real data
    #[test]
    fn test_verify_provided_chaos_labs_production_data() {
        let prices = vec![PriceData {
            feed_id: 2,
            price: 830999792250000,
            timestamp: 1743749147,
        }];

        let signature_hex = "ccfaf0d34f7f30cd607dc2a35654cdb3418df4139fcf4e73f52b98c1a279e2ca3c3a6bdc0c49781c765c809914cf832f6caffb88a1af6f6d9b17b22b1b4c42df";
        let signature_vec: [u8; 64] = {
            let vec = Vec::from_hex(signature_hex).unwrap();
            vec.try_into().expect("Hex string has incorrect length")
        };

        let batch = ChaosLabsBatchPrices {
            prices,
            signature: signature_vec,
            recovery_id: 1,
        };

        // A+KIzPZ9qCbmuJ1uaSe+njo9COAZWS3ys+u1dmNw/Vwu
        batch
            .verify_signature_with_pubkey(&[
                226, 136, 204, 246, 125, 168, 38, 230, 184, 157, 110, 105, 39, 190, 158, 58, 61, 8,
                224, 25, 89, 45, 242, 179, 235, 181, 118, 99, 112, 253, 92, 46, 143, 183, 26, 181,
                41, 16, 29, 111, 43, 44, 168, 42, 240, 165, 48, 8, 182, 246, 94, 59, 32, 196, 135,
                194, 246, 132, 210, 41, 178, 75, 112, 61,
            ])
            .unwrap(); // will panic if invalid
    }

    // Testing the signature verification with staging chaos labs real data
    #[test]
    fn test_verify_provided_chaos_labs_staging_data() {
        let prices = vec![
            PriceData {
                feed_id: 2,
                price: 834895798805000,
                timestamp: 1743651515,
            },
            PriceData {
                feed_id: 3,
                price: 833931923284500,
                timestamp: 1743651515,
            },
            PriceData {
                feed_id: 0,
                price: 1203050000000,
                timestamp: 1743651515,
            },
            PriceData {
                feed_id: 4,
                price: 112200,
                timestamp: 1743651515,
            },
            PriceData {
                feed_id: 1,
                price: 1434199529000,
                timestamp: 1743651515,
            },
        ];

        let signature_hex = "4ffef90bf04c2179c22d6022c7265684bd44081fff8c5be27d62c228e006dcff55de1dd1d7a0f173ee55c200af0f6ec96267a1361eba60dcf6e2c2fa9c12683d";
        let signature_vec: [u8; 64] = {
            let vec = Vec::from_hex(signature_hex).unwrap();
            vec.try_into().expect("Hex string has incorrect length")
        };

        let batch = ChaosLabsBatchPrices {
            prices,
            signature: signature_vec,
            recovery_id: 1,
        };

        // AzvuHexfG/y81DosUPtGfysSvF4Eoy3EKH9TMysodhjr
        batch
            .verify_signature_with_pubkey(&[
                59, 238, 29, 236, 95, 27, 252, 188, 212, 58, 44, 80, 251, 70, 127, 43, 18, 188, 94,
                4, 163, 45, 196, 40, 127, 83, 51, 43, 40, 118, 24, 235, 75, 57, 96, 128, 27, 243,
                178, 110, 146, 15, 26, 167, 103, 158, 174, 99, 89, 222, 253, 253, 4, 251, 194, 63,
                203, 86, 38, 136, 140, 229, 155, 173,
            ])
            .unwrap(); // will panic if invalid
    }

    // Testing the signature verification with production chaos labs real data with many feeds
    #[test]
    fn test_verify_provided_chaos_labs_production_data_many_feeds() {
        let prices = vec![
            PriceData {
                feed_id: 1,
                price: 2037900091500,
                timestamp: 1747111828,
            },
            PriceData {
                feed_id: 0,
                price: 1695058308300,
                timestamp: 1747111828,
            },
            PriceData {
                feed_id: 3,
                price: 1026126147718300,
                timestamp: 1747111828,
            },
            PriceData {
                feed_id: 2,
                price: 1026382743404200,
                timestamp: 1747111828,
            },
            PriceData {
                feed_id: 4,
                price: 214300,
                timestamp: 1747111828,
            },
            PriceData {
                feed_id: 5,
                price: 9998953100,
                timestamp: 1747111828,
            },
        ];

        let signature_hex = "b2790e9ff0aea631be6e5b012647d61cb8fdc425674754ccb267d3821a0ecd3d0054bebd918dff385e202f5ad2870e1084edf9ca156828be84b6bff2dc6555b4";
        let signature_vec: [u8; 64] = {
            let vec = Vec::from_hex(signature_hex).unwrap();
            vec.try_into().expect("Hex string has incorrect length")
        };

        let batch = ChaosLabsBatchPrices {
            prices,
            signature: signature_vec,
            recovery_id: 1,
        };

        // A+KIzPZ9qCbmuJ1uaSe+njo9COAZWS3ys+u1dmNw/Vwu
        batch
            .verify_signature_with_pubkey(&[
                226, 136, 204, 246, 125, 168, 38, 230, 184, 157, 110, 105, 39, 190, 158, 58, 61, 8,
                224, 25, 89, 45, 242, 179, 235, 181, 118, 99, 112, 253, 92, 46, 143, 183, 26, 181,
                41, 16, 29, 111, 43, 44, 168, 42, 240, 165, 48, 8, 182, 246, 94, 59, 32, 196, 135,
                194, 246, 132, 210, 41, 178, 75, 112, 61,
            ])
            .unwrap(); // will panic if invalid
    }

    // Testing the signature verification with production chaos labs real data with many feeds
    #[test]
    fn test_verify_provided_chaos_labs_production_data_many_feeds2() {
        let prices = vec![
            PriceData {
                feed_id: 1,
                price: 2084236754100,
                timestamp: 1747401831,
            },
            PriceData {
                feed_id: 0,
                price: 1732965243400,
                timestamp: 1747401831,
            },
            PriceData {
                feed_id: 3,
                price: 1036311677371100,
                timestamp: 1747401831,
            },
            PriceData {
                feed_id: 2,
                price: 1036774579349700,
                timestamp: 1747401831,
            },
            PriceData {
                feed_id: 4,
                price: 214600,
                timestamp: 1747401831,
            },
            PriceData {
                feed_id: 5,
                price: 9999121000,
                timestamp: 1747401831,
            },
        ];

        let signature_hex = "ffef46309eb1833f9fb01dd869b8bd5ab6f368945fff891782aa8a73aa166f5725491c52eb23e2cf1babe85156c1d123bc1850f2e9a4767c2bcd9ef413777a22";
        let signature_vec: [u8; 64] = {
            let vec = Vec::from_hex(signature_hex).unwrap();
            vec.try_into().expect("Hex string has incorrect length")
        };

        let batch = ChaosLabsBatchPrices {
            prices,
            signature: signature_vec,
            recovery_id: 0,
        };

        // A+KIzPZ9qCbmuJ1uaSe+njo9COAZWS3ys+u1dmNw/Vwu
        batch
            .verify_signature_with_pubkey(&[
                226, 136, 204, 246, 125, 168, 38, 230, 184, 157, 110, 105, 39, 190, 158, 58, 61, 8,
                224, 25, 89, 45, 242, 179, 235, 181, 118, 99, 112, 253, 92, 46, 143, 183, 26, 181,
                41, 16, 29, 111, 43, 44, 168, 42, 240, 165, 48, 8, 182, 246, 94, 59, 32, 196, 135,
                194, 246, 132, 210, 41, 178, 75, 112, 61,
            ])
            .unwrap(); // will panic if invalid
    }
}
