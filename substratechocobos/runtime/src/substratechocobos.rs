use support::{decl_storage, decl_module, decl_event, ensure, dispatch::Result,
    StorageValue, StorageMap, traits::Currency};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash, Zero};
use parity_codec::{Encode, Decode};
use rstd::cmp;
//use itertools::izip;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Chocobo<Hash, Balance> {
    id: Hash,
    dna: Hash,
    price: Balance,
    gen: u64,
    wins: u64,
    races: u64,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance,
    {
        Created(AccountId, Hash),
        PriceSet(AccountId, Hash, Balance),
        Transferred(AccountId, AccountId, Hash),
        Bought(AccountId, AccountId, Hash, Balance),
        Bred(AccountId, Hash, Hash, Hash),
        Raced(AccountId, Hash, Hash, Hash),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as ChocoboStorage {
        Chocobos get(choco_by_id): map T::Hash => Chocobo<T::Hash, T::Balance>;
        Owners get(owner_of): map T::Hash => Option<T::AccountId>;

        AllChocobosArray get(choco_by_index): map u64 => T::Hash;
        AllChocobosCount get(get_all_count): u64;
        AllChocobosIndex: map T::Hash => u64;

        OwnedChocobosArray get(choco_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnedChocobosCount get(count_by_account): map T::AccountId => u64;
        OwnedChocobosIndex: map T::Hash => u64;
        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        fn create_chocobo(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let nonce = <Nonce<T>>::get();
            let random_seed = <system::Module<T>>::random_seed();
            let random_hash = (random_seed, &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let new_choco = Chocobo {
                id: random_hash,
                dna: random_hash,
                price: <T::Balance as As<u64>>::sa(0),
                gen: 0,
                wins: 0,
                races: 0,
            };

            Self::mint(sender, random_hash, new_choco)?;
            <Nonce<T>>::mutate(|n| *n += 1);
            Ok(())
        }

        fn set_price(origin, choco_id: T::Hash, new_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(<Chocobos<T>>::exists(choco_id), "This choco does not exist");

            let owner = Self::owner_of(choco_id).ok_or("No owner for this chocobo")?;
            ensure!(owner == sender, "You do not own this chocobo");

            let mut choco = Self::choco_by_id(choco_id);
            choco.price = new_price;

            <Chocobos<T>>::insert(choco_id, choco);

            Self::deposit_event(RawEvent::PriceSet(sender, choco_id, new_price));
            Ok(())
        }

        fn transfer(origin, to: T::AccountId, choco_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            let owner = Self::owner_of(choco_id).ok_or("No owner of this chocobo")?;
            ensure!(owner == sender, "You do not own this chocobo");

            Self::transfer_from(sender, to, choco_id)?;

            Ok(())
        }

        fn buy_chocobo(origin, choco_id: T::Hash, max_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(<Chocobos<T>>::exists(choco_id), "This chocobo does not exist");

            let owner = Self::owner_of(choco_id).ok_or("No owner for this chocobo")?;
            ensure!(owner != sender, "You already own this chocobo");

            let mut choco = Self::choco_by_id(choco_id);
            let price = choco.price;
            ensure!(!price.is_zero(), "The chocobo you want is not for sale");
            ensure!(price <= max_price, "The chocobo you want costs more than your max price");

            <balances::Module<T> as Currency<_>>::transfer(&sender, &owner, price)?;
            Self::transfer_from(owner.clone(), sender.clone(), choco_id)
                .expect("`owner` shown to own chocobo; \
                         `owner` has at least 1 kitten so transfer cannot underflow; \
                         `owner_count` shares type with `all_count` \
                         and minting ensures lt `max()` chocobos \
                         so transfer cannot overflow; \
                         qed");

            choco.price = <T::Balance as As<u64>>::sa(0);
            <Chocobos<T>>::insert(choco_id, choco);

            Self::deposit_event(RawEvent::Bought(sender, owner, choco_id, price));
            Ok(())
        }

        fn breed_chocobo(origin, sire_id: T::Hash, mare_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(<Chocobos<T>>::exists(sire_id), "Sire chocobo does not exist");
            ensure!(<Chocobos<T>>::exists(mare_id), "Mare chocobo does not exist");

            let nonce = <Nonce<T>>::get();
            let random_seed = <system::Module<T>>::random_seed();
            let random_hash = (random_seed, &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let sire = Self::choco_by_id(sire_id);
            let mare = Self::choco_by_id(mare_id);

            let mut child_dna = sire.dna;
            for (i, (mare_genotype, rand)) in
              mare.dna.as_ref().iter().zip(random_hash.as_ref().iter()).enumerate() {
                if rand % 2 == 0 {
                    child_dna.as_mut()[i] = *mare_genotype;
                }
            }

            let new_choco = Chocobo {
                id: random_hash,
                dna: child_dna,
                price: <T::Balance as As<u64>>::sa(0),
                gen: cmp::max(sire.gen, mare.gen) + 1,
                wins: 0,
                races: 0,
            };

            Self::mint(sender.clone(), random_hash, new_choco)?;
            <Nonce<T>>::mutate(|n| *n += 1);
            Self::deposit_event(RawEvent::Bred(sender, sire_id, mare_id, random_hash));
            Ok(())
        }

        fn race(origin, choco1_id: T::Hash, choco2_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(<Chocobos<T>>::exists(choco1_id), "Chocobo contender1 does not exist");
            ensure!(<Chocobos<T>>::exists(choco2_id), "Chocobo contender1 does not exist");

            //let nonce = <Nonce<T>>::get();
            //let random_seed = <system::Module<T>>::random_seed();
            //let random_hash = (random_seed, &sender, nonce)
            //    .using_encoded(<T as system::Trait>::Hashing::hash);

            let mut choco1 = Self::choco_by_id(choco1_id);
            let dna1 = choco1.dna.as_ref().iter();
            let mut choco2 = Self::choco_by_id(choco2_id);
            let dna2 = choco2.dna.as_ref().iter();

            let mut winner = choco1_id;
            let mut outcome = 0;
            //for (gt1, gt2, rand) in izip!(dna1, dna2, random_hash) {
            for (gt1, gt2) in dna1.zip(dna2) {
                if gt1 >= gt2 {
                    outcome += 1;
                } else {
                    outcome -= 1;
                }
            }

            choco1.races += 1; //checked_add
            choco2.races += 1; //checked_add
            if outcome >= 0 {
                choco1.wins += 1; //checked_add
            } else {
                winner = choco2_id;
                choco2.wins += 1; //checked_add
            }
            <Chocobos<T>>::insert(choco1_id, choco1);
            <Chocobos<T>>::insert(choco2_id, choco2);

            <Nonce<T>>::mutate(|n| *n += 1);
            Self::deposit_event(RawEvent::Raced(sender, choco1_id, choco2_id, winner));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn mint(to: T::AccountId, choco_id: T::Hash, new_choco: Chocobo<T::Hash, T::Balance>) -> Result {
        ensure!(!<Chocobos<T>>::exists(choco_id), "This new chocobo id already exists");

        let owned_count = Self::count_by_account(&to);
        let new_owned_count = owned_count.checked_add(1)
            .ok_or("Overflow adding a new chocobo to account")?;

        let all_count = Self::get_all_count();
        let new_count = all_count.checked_add(1)
            .ok_or("Overflow adding a new chocobo to total")?;

        <Chocobos<T>>::insert(choco_id, new_choco);
        <Owners<T>>::insert(choco_id, &to);

        <AllChocobosArray<T>>::insert(all_count, choco_id);
        <AllChocobosCount<T>>::put(new_count);
        <AllChocobosIndex<T>>::insert(choco_id, all_count);

        <OwnedChocobosArray<T>>::insert((to.clone(), owned_count), choco_id);
        <OwnedChocobosCount<T>>::insert(&to, new_owned_count);
        <OwnedChocobosIndex<T>>::insert(choco_id, owned_count);

        Self::deposit_event(RawEvent::Created(to, choco_id));
        Ok(())
    }

    fn transfer_from(from: T::AccountId, to: T::AccountId, choco_id: T::Hash) -> Result {
        let owner = Self::owner_of(choco_id).ok_or("No owner of this chocobo")?;
        ensure!(owner == from, "You do not own this chocobo");

        let owned_count_from = Self::count_by_account(&from);
        let owned_count_to = Self::count_by_account(&to);
        let new_count_from = owned_count_from.checked_sub(1)
            .ok_or("Transfer causes underflow of 'from' account")?;
        let new_count_to = owned_count_to.checked_add(1)
            .ok_or("Transfer causes overflow of 'to' account")?;

        let choco_index = <OwnedChocobosIndex<T>>::get(choco_id);
        if choco_index != new_count_from {
            let last_choco_id = <OwnedChocobosArray<T>>::get((from.clone(), new_count_from));
            <OwnedChocobosArray<T>>::insert((from.clone(), choco_index), last_choco_id);
            <OwnedChocobosIndex<T>>::insert(last_choco_id, choco_index);
        }

        <Owners<T>>::insert(choco_id, &to);
        <OwnedChocobosIndex<T>>::insert(choco_id, owned_count_to);
        <OwnedChocobosArray<T>>::remove((from.clone(), new_count_from));
        <OwnedChocobosArray<T>>::insert((to.clone(), owned_count_to), choco_id); 

        <OwnedChocobosCount<T>>::insert(&from, new_count_from);
        <OwnedChocobosCount<T>>::insert(&to, new_count_to);

        Self::deposit_event(RawEvent::Transferred(from, to, choco_id));
        Ok(())
    }
}
