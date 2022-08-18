use scrypto::prelude::*;
use crate::lending_app::LendingApp;
use crate::trading_app::TradingApp;
use crate::utils::*;

const FIXED_FEE: i32 = 10;

// Here, we define the data that will be present in
// each of the user . 
#[derive(NonFungibleData)]
struct UserHistory {
    #[scrypto(mutable)]
    username: ComponentAddress,
    #[scrypto(mutable)]   
    positive_operation: u32,    
    #[scrypto(mutable)]   
    negative_operation: u32, 
    #[scrypto(mutable)]   
    expert: bool          
}

// definition of operation 
#[derive(TypeId, Encode, Decode, Describe)]
struct OperationHistory {
    username: ComponentAddress,
    operation_id: u128,    
    xrd_tokens: Decimal,    
    current_price: u64,
    token_a_address: ResourceAddress, 
    token_b_address: ResourceAddress,
    num_token_b_received: Decimal,
    date_opened: u64, 
    date_closed: Option<u64>,     
    current_standing: Option<bool>,    
    number_of_request_for_autoclosing: Option<u32>,  
    current_requestor_for_closing: Option<ResourceAddress>   
}

impl ToString for OperationHistory {
    fn to_string(&self) -> String {
        return format!("{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}", 
        self.username,
        self.operation_id,    
        self.xrd_tokens,  
        self.current_price,  
        self.token_a_address,    
        self.token_a_address,    
        self.num_token_b_received,
        self.date_opened, 
        self.date_closed,     
        self.current_standing,    
        self.number_of_request_for_autoclosing,
        self.current_requestor_for_closing
        );
    }
}

blueprint!{
    /// The Portfolio blueprint 
    struct Portfolio{

        /// The reserve for main pool
        main_pool: Vault,

        /// The reserve for trading token1 main pool
        token1_pool: Vault,

        lending_app: ComponentAddress,

        trading_app: ComponentAddress,

        /// The resource definition of UserHistory token.
        username_nft_resource_def: ResourceAddress,
        /// Vault with admin badge for managine UserHistory NFT.
        username_nft_admin_badge: Vault,    
        
        //positions opened/closed
        positions: Vec<OperationHistory>,

        lending_nft_vault: Vault,
    }

    

    // resim call-function $package TradingApp create_market $xrd $btc $eth $leo
//procedo con il funding del market
// resim call-method $component fund_market 1000,$xrd 1000,$btc 1000,$eth 1000,$leo

    impl Portfolio {
        /// Instantiates a new Portfolio component. 
        pub fn new(
            xrd_address: ResourceAddress, 
            token_1_address: ResourceAddress,
            lending_app: ComponentAddress,
            trading_app: ComponentAddress,
            lending_nft_resource_def: ResourceAddress) -> ComponentAddress {

            // let rules = AccessRules::new()
            // .method("issue_new_credit_sbt", rule!(require(admin_badge)))
            // .method("review_installment_credit_request", rule!(require(admin_badge)))
            // .method("list_protocol", rule!(require(admin_badge)))
            // .method("delist_protocol", rule!(require(admin_badge)))
            // .method("blacklist", rule!(require(admin_badge)))
            // .method("whitelist", rule!(require(admin_badge)))
            // .method("change_credit_scoring_rate", rule!(require(admin_badge)))
            // .default(rule!(allow_all));

            let user_mint_badge = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "User Mint Badge")
                .initial_supply(1);

            let username_nft = ResourceBuilder::new_non_fungible()
                .metadata("name", "Username History")
                .mintable(rule!(require(user_mint_badge.resource_address())), LOCKED)
                .burnable(rule!(require(user_mint_badge.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(user_mint_badge.resource_address())), LOCKED)
                .no_initial_supply();

            return Self {
                main_pool: Vault::new(xrd_address),
                token1_pool: Vault::new(token_1_address),
                lending_app: lending_app,
                trading_app: trading_app,
                username_nft_resource_def: username_nft,
                username_nft_admin_badge: Vault::with_bucket(user_mint_badge),
                positions: Vec::new(),
                lending_nft_vault: Vault::new(lending_nft_resource_def),
            }
            .instantiate()
            // .add_access_check(rules)
            .globalize();            
        }



        pub fn register(&mut self,address: ComponentAddress) -> Bucket {
            let nft = self.username_nft_admin_badge.authorize(|| {
                let resource_manager = borrow_resource_manager!(self.username_nft_resource_def);
                resource_manager.mint_non_fungible(
                    // The NFT id
                    &get_non_fungible_id(),
                    // The NFT data
                    UserHistory {
                        username: address,
                        positive_operation: 0,
                        negative_operation: 0,
                        expert: false,
                    },
                )
            });
            nft
        }

        pub fn fund_portfolio(&mut self, starting_tokens: Bucket, ticket: Proof) {
            info!("=== FUND PORTFOLIO OPERATION START === ");
                self.main_pool.put(starting_tokens);
        }
        pub fn withdraw_portfolio(&mut self, tokens_to_withdraw: Decimal) {
            info!("=== WITHDRAW PORTFOLIO OPERATION START === ");
                self.main_pool.take(tokens_to_withdraw);
        }

       /// # Execute a buy operation by means of the portfolio.
       pub fn buy(&mut self,xrd_tokens: Decimal, user_account: ComponentAddress, token_to_buy: ResourceAddress)   {
            assert!(
                self.main_pool.amount() >= xrd_tokens,
                "Main vault has not sufficient tokens to buy ! Please fund portfolio !"
            );   
            let trading_app: TradingApp = self.trading_app.into();
            self.token1_pool.put(trading_app.buy(self.main_pool.take(xrd_tokens)));
            
            let current_price = trading_app.current_price(RADIX_TOKEN,token_to_buy);
            let how_many = xrd_tokens / current_price;

            let trade1 = OperationHistory {
                username: user_account,
                operation_id: Runtime::generate_uuid(),    
                xrd_tokens: xrd_tokens,    
                current_price: current_price,    
                token_a_address: RADIX_TOKEN,
                token_b_address: token_to_buy,
                num_token_b_received: how_many,
                date_opened: Runtime::current_epoch(),
                date_closed: None,
                current_requestor_for_closing: None, 
                current_standing: None,
                number_of_request_for_autoclosing: None,
            };

            self.positions.push(trade1);
        }

        pub fn sell(&mut self,tokens: Decimal
        )   {
            let trading_app: TradingApp = self.trading_app.into();
            self.main_pool.put(trading_app.sell(self.token1_pool.take(tokens)));
        }
   

        pub fn position(&self) -> Vec<u128> {
            info!("Position size {}", self.positions.len());
            let trading_app: TradingApp = self.trading_app.into();

            let mut losing_positions: Vec<u128> = Vec::new();
            let mut result: Decimal = Decimal::zero();
            for inner_position in &self.positions {
                info!("Inner Position {}", inner_position.to_string());    
                info!("Position Id {}", inner_position.operation_id);          

                info!("Ready to get current price ");
                let updated_value = trading_app.current_price(inner_position.token_a_address,inner_position.token_b_address);
                info!("Xrd used for trade {}", inner_position.xrd_tokens);
                info!("Starting price {:?}", inner_position.current_price );
                info!("Current price {:?}", updated_value );
                let net_result = updated_value.wrapping_sub(inner_position.current_price);
                info!("Position net result {:?}", net_result);

                if net_result >= 0 {
                    let trade1 = OperationHistory {
                        username: inner_position.username,
                        operation_id: inner_position.operation_id,    
                        xrd_tokens: inner_position.xrd_tokens,    
                        current_price: inner_position.current_price,    
                        token_a_address: RADIX_TOKEN,
                        token_b_address: inner_position.token_b_address,
                        num_token_b_received: inner_position.num_token_b_received,
                        date_opened: inner_position.date_opened,
                        date_closed: None,
                        current_requestor_for_closing: None, 
                        current_standing: None,
                        number_of_request_for_autoclosing: None,
                    };
                    losing_positions.push(inner_position.operation_id);
                };

            }        

            losing_positions
        }

        pub fn close_position(&mut self, operation_id: u128)  {
            info!("Position size {}", self.positions.len());
            let mut amount_to_sell: Decimal = Decimal::zero();
            for inner_position in &self.positions {
                info!("Inner Position {}", inner_position.to_string());           
                info!("Position Id {}", inner_position.operation_id);    

                if inner_position.operation_id==operation_id {
                    amount_to_sell = inner_position.num_token_b_received.clone();
                }
            }    
            info!("Ready to close position {}", operation_id);
            self.sell(amount_to_sell);    
        }


        pub fn register_for_lending(&mut self)  {
            info!("Registering for lending ") ;
            info!("Vault for Lending NFT, accept resource address : {:?} ", self.lending_nft_vault.resource_address());
            let lending_app: LendingApp = self.lending_app.into();
            let bucket: Bucket = lending_app.register();
            self.lending_nft_vault.put(bucket);
        }
        pub fn register_for_lending_old(&mut self) -> Bucket {
            info!("Registering for lending ") ;
            let lending_app: LendingApp = self.lending_app.into();
            return lending_app.register();
        }
        pub fn register_for_borrowing(&mut self) -> Bucket {
            info!("Registering for borrowing ") ;
            let lending_app: LendingApp = self.lending_app.into();
            return lending_app.register_borrower();
        }     

        pub fn lend(&mut self,tokens: Bucket) -> Bucket {
            info!("Lending ");
            let lending_app: LendingApp = self.lending_app.into();
            let proof: Proof = self.lending_nft_vault.create_proof_by_amount(dec!(1));
            return lending_app.lend_money(tokens, proof);
        }

        pub fn take_back(&mut self, lnd_tokens: Bucket) -> Bucket {
            info!("Take back ");
            let lending_app: LendingApp = self.lending_app.into();
            let proof: Proof = self.lending_nft_vault.create_proof_by_amount(dec!(1));
            return lending_app.take_money_back(lnd_tokens, proof);
        }

            // This is a pseudorandom function and not a true random number function.
    // pub fn get_random(&self) -> u128 {
    //     let multiplier = self.players.clone().into_iter()
    //         .map(|(_,p)| p.guess)
    //         .reduce(|a,b| a * b).unwrap_or(1);

    //     Runtime::generate_uuid() / multiplier
    // }

    // let random_number = (self.get_random() % 6) + 1;
    

    }
}