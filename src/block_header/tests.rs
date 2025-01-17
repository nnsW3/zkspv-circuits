use crate::{
    keccak::FnSynthesize,
    keccak::SharedKeccakChip,
    util::{EthConfigParams, EthConfigPinning, Halo2ConfigPinning},
};

use super::*;
use ark_std::{end_timer, start_timer};
use ethers_core::utils::hex::FromHex;
use halo2_base::{
    halo2_proofs::{
        dev::MockProver,
        halo2curves::bn256::{Bn256, Fr, G1Affine},
        plonk::*,
        poly::commitment::ParamsProver,
        poly::kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::{ProverSHPLONK, VerifierSHPLONK},
            strategy::SingleStrategy,
        },
        transcript::{
            Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
        },
    },
    utils::fs::gen_srs,
};
use rand_core::OsRng;
use rayon::ThreadPoolBuilder;
use std::{
    env::{set_var, var},
    fs::File,
};
use test_log::test;

fn block_header_test_circuit<F: Field>(
    mut builder: RlcThreadBuilder<F>,
    inputs: Vec<Vec<u8>>,
    network: Network,
    break_points: Option<RlcThreadBreakPoints>,
) -> EthCircuitBuilder<F, impl FnSynthesize<F>> {
    let prover = builder.witness_gen_only();
    let range = RangeChip::default(ETH_LOOKUP_BITS);
    let keccak = SharedKeccakChip::default();
    let chip = EthChip::new(RlpChip::new(&range, None), None);
    let block_header_config = get_block_header_config(&network);
    let chain_witness = chip.decompose_block_header_chain_phase0(
        &mut builder.gate_builder,
        &mut keccak.borrow_mut(),
        inputs,
        &block_header_config,
    );

    let circuit = EthCircuitBuilder::new(
        vec![],
        builder,
        keccak,
        range,
        break_points,
        move |builder: &mut RlcThreadBuilder<F>,
              rlp: RlpChip<F>,
              keccak_rlcs: (FixedLenRLCs<F>, VarLenRLCs<F>)| {
            let chip = EthChip::new(rlp, Some(keccak_rlcs));
            let _block_chain_trace =
                chip.decompose_block_header_chain_phase1(builder, chain_witness, None);
        },
    );
    if !prover {
        let config_params: EthConfigParams = serde_json::from_str(
            var("ETH_CONFIG_PARAMS").expect("ETH_CONFIG_PARAMS is not set").as_str(),
        )
        .unwrap();
        circuit.config(config_params.degree as usize, Some(config_params.unusable_rows));
    }
    circuit
}

#[test]
pub fn test_one_mainnet_header_mock() {
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Ethereum(EthereumNetwork::Mainnet);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90201a0d7519abd494a823b2c9c28908eaf250fe4a6287d747f1cc53a5a193b6533a549a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944675c7e5baafbffbca748158becba61ef3b0a263a025000d51f040ee5c473fed74eda9ace87d55a35187b11bcde6f5176025c395bfa0a5800a6de6d28d7425ff72714af2af769b9f8f9e1baf56fb42f793fbb40fde07a056e1062a3dc63791e8a8496837606b14062da70ee69178cea97d6eeb5047550cb9010000236420014dc00423903000840002280080282100004704018340c0241c20011211400426000f900001d8088000011006020002ce98bc00c0000020c9a02040000688040200348c3a0082b81402002814922008085d008008200802802c4000130000101703124801400400018008a6108002020420144011200070020bc0202681810804221304004800088600300000040463614a000e200201c00611c0008e800b014081608010a0218a0b410010082000428209080200f50260a00840006700100f40a000000400000448301008c4a00341040e343500800d06250020010215200c008018002c88350404000bc5000a8000210c00724a0d0a4010210a448083eee2468401c9c3808343107884633899e780a07980d8d1f15474c9185e4d1cef5f207167735009daad2eb6af6da37ffba213c28800000000000000008501e08469e60000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let circuit =
        block_header_test_circuit::<Fr>(RlcThreadBuilder::mock(), vec![input_bytes], network, None);
    MockProver::run(k, &circuit, vec![vec![]]).unwrap().assert_satisfied();
}

#[test]
pub fn test_one_mainnet_header_before_london_mock() {
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Ethereum(EthereumNetwork::Mainnet);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90221a0b8b861952bca93c10bc7c38f9ef5c4e047beae539cfe46fa456c78893d916927a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347940501b62d81a3f072f1d393d2f74013bab8d36d5ca01fd1d6a626d5d72d433b776c0c348f0cab03d13c68ba39ca4a6d6f109032de34a0418c7fdf567a5989a727ea0fe6054008ecf4953aaf56c28f7f197f6e443f05c0a05f79bcb9839eb480350b541377d04c5088fc4bab6952ed27cb94c70dd6736d73b9010081029040054830208119a218064a503c384490dc2014a414e3148820851856c05008e643a88a4a0002242e1a702d8a516244220a18cd0121a13a20882930000e471369c142ad4323475013088accb068824a002cc35021640860a448405a904001094c200a6081d0420feb02802c2e090a121403213d2640c100503510300364e43020f55943142815080595b145040045890021412545119b9002891cfe41011a704100ca97641210002a3b22c10f24853849048420100465c361880421593000021022c90800008800750e546464068cc40290108c48741899114af9c52801403da6800c02000c6ea270992068b45618c46f1254d7601d4411104e41d00a0787074abe0f14de3383765fdd837a121d8379cbd7845cda8ef39fde830203088f5061726974792d457468657265756d86312e33332e30826c69a09d41f9f64af4ebd672dec132507a12a4c85c1a514f47969dbd9c2b5e9d7d214e882b8a10229542325400000000000000000000";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let circuit =
        block_header_test_circuit::<Fr>(RlcThreadBuilder::mock(), vec![input_bytes], network, None);
    MockProver::run(k, &circuit, vec![vec![]]).unwrap().assert_satisfied();
}

#[test]
pub fn test_one_mainnet_header_withdrawals_mock() {
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Ethereum(EthereumNetwork::Mainnet);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90222a0d7519abd494a823b2c9c28908eaf250fe4a6287d747f1cc53a5a193b6533a549a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944675c7e5baafbffbca748158becba61ef3b0a263a025000d51f040ee5c473fed74eda9ace87d55a35187b11bcde6f5176025c395bfa0a5800a6de6d28d7425ff72714af2af769b9f8f9e1baf56fb42f793fbb40fde07a056e1062a3dc63791e8a8496837606b14062da70ee69178cea97d6eeb5047550cb9010000236420014dc00423903000840002280080282100004704018340c0241c20011211400426000f900001d8088000011006020002ce98bc00c0000020c9a02040000688040200348c3a0082b81402002814922008085d008008200802802c4000130000101703124801400400018008a6108002020420144011200070020bc0202681810804221304004800088600300000040463614a000e200201c00611c0008e800b014081608010a0218a0b410010082000428209080200f50260a00840006700100f40a000000400000448301008c4a00341040e343500800d06250020010215200c008018002c88350404000bc5000a8000210c00724a0d0a4010210a448083eee2468401c9c3808343107884633899e780a07980d8d1f15474c9185e4d1cef5f207167735009daad2eb6af6da37ffba213c28800000000000000008501e08469e6a0f7519abd494a823b2c9c28908eaf250fe4a6287d747f1cc53a5a193b6533a549";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let circuit =
        block_header_test_circuit::<Fr>(RlcThreadBuilder::mock(), vec![input_bytes], network, None);
    MockProver::run(k, &circuit, vec![vec![]]).unwrap().assert_satisfied();
}

#[test]
pub fn test_one_mainnet_header_prover() -> Result<(), Box<dyn std::error::Error>> {
    ThreadPoolBuilder::new().num_threads(256).build_global().unwrap();
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Ethereum(EthereumNetwork::Mainnet);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90222a0d7519abd494a823b2c9c28908eaf250fe4a6287d747f1cc53a5a193b6533a549a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944675c7e5baafbffbca748158becba61ef3b0a263a025000d51f040ee5c473fed74eda9ace87d55a35187b11bcde6f5176025c395bfa0a5800a6de6d28d7425ff72714af2af769b9f8f9e1baf56fb42f793fbb40fde07a056e1062a3dc63791e8a8496837606b14062da70ee69178cea97d6eeb5047550cb9010000236420014dc00423903000840002280080282100004704018340c0241c20011211400426000f900001d8088000011006020002ce98bc00c0000020c9a02040000688040200348c3a0082b81402002814922008085d008008200802802c4000130000101703124801400400018008a6108002020420144011200070020bc0202681810804221304004800088600300000040463614a000e200201c00611c0008e800b014081608010a0218a0b410010082000428209080200f50260a00840006700100f40a000000400000448301008c4a00341040e343500800d06250020010215200c008018002c88350404000bc5000a8000210c00724a0d0a4010210a448083eee2468401c9c3808343107884633899e780a07980d8d1f15474c9185e4d1cef5f207167735009daad2eb6af6da37ffba213c28800000000000000008501e08469e6a0f7519abd494a823b2c9c28908eaf250fe4a6287d747f1cc53a5a193b6533a549";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let mut rng = OsRng;
    let params = ParamsKZG::<Bn256>::setup(k, &mut rng);
    let circuit = block_header_test_circuit::<Fr>(
        RlcThreadBuilder::keygen(),
        vec![input_bytes.clone()],
        network,
        None,
    );
    let vk_time = start_timer!(|| "vk gen");
    let vk = keygen_vk(&params, &circuit).unwrap();
    end_timer!(vk_time);
    let pk_time = start_timer!(|| "pk gen");
    let pk = keygen_pk(&params, vk, &circuit).unwrap();
    end_timer!(pk_time);
    let break_points = circuit.circuit.break_points.take();
    let pinning = EthConfigPinning {
        params: serde_json::from_str(var("ETH_CONFIG_PARAMS").unwrap().as_str()).unwrap(),
        break_points,
    };
    serde_json::to_writer(File::create("configs/tests/one_block.json").unwrap(), &pinning)?;

    let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    let pf_time = start_timer!(|| "proof gen");
    let break_points = pinning.break_points();
    let circuit = block_header_test_circuit::<Fr>(
        RlcThreadBuilder::prover(),
        vec![input_bytes],
        network,
        Some(break_points),
    );
    create_proof::<
        KZGCommitmentScheme<Bn256>,
        ProverSHPLONK<'_, Bn256>,
        Challenge255<G1Affine>,
        _,
        Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>>,
        _,
    >(&params, &pk, &[circuit], &[&[&[]]], rng, &mut transcript)?;
    let proof = transcript.finalize();
    end_timer!(pf_time);

    let verifier_params = params.verifier_params();
    let strategy = SingleStrategy::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    let verify_time = start_timer!(|| "verify");
    verify_proof::<
        KZGCommitmentScheme<Bn256>,
        VerifierSHPLONK<'_, Bn256>,
        Challenge255<G1Affine>,
        Blake2bRead<&[u8], G1Affine, Challenge255<G1Affine>>,
        SingleStrategy<'_, Bn256>,
    >(verifier_params, pk.get_vk(), strategy, &[&[&[]]], &mut transcript)
    .unwrap();
    end_timer!(verify_time);

    Ok(())
}

fn get_default_goerli_header_chain_circuit() -> EthBlockHeaderChainCircuit<Fr> {
    let network = Network::Ethereum(EthereumNetwork::Goerli);
    let block_header_config = get_block_header_config(&network);
    let blocks: Vec<String> =
        serde_json::from_reader(File::open("data/headers/default_blocks_goerli.json").unwrap())
            .unwrap();
    let mut input_bytes = Vec::new();
    let max_depth = 3;
    for block_str in blocks.iter() {
        let mut block_vec: Vec<u8> = Vec::from_hex(block_str).unwrap();
        block_vec.resize(block_header_config.block_header_rlp_max_bytes, 0);
        input_bytes.push(block_vec);
    }
    let dummy_header_rlp = input_bytes[0].clone();
    input_bytes.extend(iter::repeat(dummy_header_rlp).take((1 << max_depth) - input_bytes.len()));

    EthBlockHeaderChainCircuit {
        header_rlp_encodings: input_bytes,
        num_blocks: 7,
        max_depth,
        block_header_config,
        _marker: PhantomData,
    }
}

#[test]
pub fn test_multi_goerli_header_mock() {
    let config = EthConfigPinning::from_path("configs/tests/multi_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&config).unwrap());
    let k = config.degree;

    let input = get_default_goerli_header_chain_circuit();
    let circuit = input.create_circuit(RlcThreadBuilder::mock(), None);
    let instance = circuit.instance();

    MockProver::run(k, &circuit, vec![instance]).unwrap().assert_satisfied();
}

#[test]
pub fn test_multi_goerli_header_prover() {
    let config = EthConfigPinning::from_path("configs/tests/multi_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&config).unwrap());
    let k = config.degree;
    let input = get_default_goerli_header_chain_circuit();
    let circuit = input.clone().create_circuit(RlcThreadBuilder::keygen(), None);

    let params = gen_srs(k);

    let vk_time = start_timer!(|| "vk gen");
    let vk = keygen_vk(&params, &circuit).unwrap();
    end_timer!(vk_time);
    let pk_time = start_timer!(|| "pk gen");
    let pk = keygen_pk(&params, vk, &circuit).unwrap();
    end_timer!(pk_time);
    let break_points = circuit.circuit.break_points.take();
    let pinning = EthConfigPinning {
        params: serde_json::from_str(var("ETH_CONFIG_PARAMS").unwrap().as_str()).unwrap(),
        break_points,
    };
    serde_json::to_writer(File::create("configs/tests/multi_block.json").unwrap(), &pinning)
        .unwrap();

    let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    let pf_time = start_timer!(|| "proof gen");
    let break_points = pinning.break_points();
    let circuit = input.create_circuit(RlcThreadBuilder::prover(), Some(break_points));
    let instance = circuit.instance();
    create_proof::<
        KZGCommitmentScheme<Bn256>,
        ProverSHPLONK<'_, Bn256>,
        Challenge255<G1Affine>,
        _,
        Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>>,
        _,
    >(&params, &pk, &[circuit], &[&[&instance]], OsRng, &mut transcript)
    .unwrap();
    let proof = transcript.finalize();
    end_timer!(pf_time);

    let verifier_params = params.verifier_params();
    let strategy = SingleStrategy::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    let verify_time = start_timer!(|| "verify");
    verify_proof::<
        KZGCommitmentScheme<Bn256>,
        VerifierSHPLONK<'_, Bn256>,
        Challenge255<G1Affine>,
        Blake2bRead<&[u8], G1Affine, Challenge255<G1Affine>>,
        SingleStrategy<'_, Bn256>,
    >(verifier_params, pk.get_vk(), strategy, &[&[&instance]], &mut transcript)
    .unwrap();
    end_timer!(verify_time);
}

#[cfg(all(feature = "aggregation", feature = "providers"))]
mod aggregation {
    use std::path::PathBuf;

    use super::test;
    use super::*;
    use crate::block_header::helper::{BlockHeaderScheduler, CircuitType, Finality, Task};
    use crate::util::scheduler::Scheduler;

    fn test_scheduler(network: Network) -> BlockHeaderScheduler {
        BlockHeaderScheduler::new(
            network,
            false,
            false,
            PathBuf::from("configs/headers"),
            PathBuf::from("data/headers"),
            PathBuf::from("cache_data/headers"),
        )
    }

    #[test]
    fn test_goerli_header_chain_provider() {
        let scheduler = test_scheduler(Network::Ethereum(EthereumNetwork::Goerli));
        scheduler.get_snark(Task::new(
            0x765fb3,
            0x765fb3 + 7,
            CircuitType::new(3, 3, Finality::None, Network::Ethereum(EthereumNetwork::Goerli)),
        ));
    }

    #[test]
    #[ignore = "requires over 32G memory"]
    fn test_goerli_header_chain_with_aggregation() {
        let scheduler = test_scheduler(Network::Ethereum(EthereumNetwork::Goerli));
        scheduler.get_snark(Task::new(
            0x765fb3,
            0x765fb3 + 11,
            CircuitType::new(4, 3, Finality::None, Network::Ethereum(EthereumNetwork::Goerli)),
        ));
    }

    #[test]
    #[ignore = "requires over 32G memory"]
    fn test_goerli_header_chain_final_aggregation() {
        let scheduler = test_scheduler(Network::Ethereum(EthereumNetwork::Goerli));
        scheduler.get_snark(Task::new(
            0x765fb3,
            0x765fb3 + 9,
            CircuitType::new(4, 3, Finality::Merkle, Network::Ethereum(EthereumNetwork::Goerli)),
        ));
    }

    #[cfg(feature = "evm")]
    #[test]
    fn test_goerli_header_chain_for_evm() {
        let scheduler = test_scheduler(Network::Ethereum(EthereumNetwork::Goerli));
        scheduler.get_calldata(
            Task::new(
                0x765fb3,
                0x765fb3 + 1,
                CircuitType::new(
                    4,
                    3,
                    Finality::Evm(1),
                    Network::Ethereum(EthereumNetwork::Goerli),
                ),
            ),
            true,
        );
    }
}

#[test]
pub fn test_one_arbitrum_goerli_header_mock() {
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Arbitrum(ArbitrumNetwork::Goerli);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90223a03d156e87db35e1e3e0ee5e535665ac1f03f7617387e8cd6dd3f9693ab7a2361da01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d4934794a4b000000000000000000073657175656e636572a03b1567d78a408b60902348ab3fe08e4cbec17b2ee0c43192d67d380e1c03db89a055397ee2af918be0697ccb41d534a3a238e4823f69d32a71d6c3bed830b7d441a065dcb404f4eac4e1a7e91410f2b59ae2b07b65f23680a4009880de8bc8221c01b90100000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000010000000000000000000000000000000000000000000000000183cad6208704000000000000831976f184641d58f1a001654e9e6b8851a72f60a96e6c6c6fb6e3b34e88e0a5352020cc75ddf5c11633a00000000000006c3c000000000084e318000000000000000a000000000000000088000000000004a5938408f453f0";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let circuit =
        block_header_test_circuit::<Fr>(RlcThreadBuilder::mock(), vec![input_bytes], network, None);
    MockProver::run(k, &circuit, vec![vec![]]).unwrap().assert_satisfied();
}

#[test]
pub fn test_one_arbitrum_mainnet_header_mock() {
    let params = EthConfigPinning::from_path("configs/tests/one_block.json").params;
    set_var("ETH_CONFIG_PARAMS", serde_json::to_string(&params).unwrap());
    let network = Network::Arbitrum(ArbitrumNetwork::Mainnet);
    let config = get_block_header_config(&network);
    let k = params.degree;
    let input_hex = "f90224a0fff1d5d4d2ca04565a774745c2b8cec7f494e6772df22e57256ea1faa11dcdf9a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d4934794a4b000000000000000000073657175656e636572a0629e9d752730e69aa42d828c0176a33f803e58fa8686a03c69c116119dfb7b65a0894d1d0afd5d7087e01d5bf3ae368bc63fd8a8916c04ca803e5415d2ae351b41a01f8ea6377a287afacc92f960777f1778c449b813e3970f20ef34207e34ec3eb8b9010000240000000010000000000080200000000000001000000000000100001005101000000000000400000000000000000020094200000020000010000000280108000000110000000800000028100100200000000000000008000002000000000840040400020000000000000400000a200000000000084080000000100408500820000000000000000000150000000000000000030008000800000040000000120208023000002000180300000100000000010a000000100000000000000000010000040200040000000000000012000100000002000020100400000000002100041000000400000000080000a0000000080000000100000010000000000000020184056ec5dc8704000000000000836414d1846462f54ba0bae71d65d442f7199d75900794f44fc1ff9bb4169d2d5ef079f58db79928820da000000000000127dd000000000107832d000000000000000a00000000000000008800000000000d01b98405f5e100";
    let mut input_bytes: Vec<u8> = Vec::from_hex(input_hex).unwrap();
    input_bytes.resize(config.block_header_rlp_max_bytes, 0);

    let circuit = block_header_test_circuit::<Fr>(
        RlcThreadBuilder::mock(),
        vec![input_bytes],
        Network::Arbitrum(ArbitrumNetwork::Mainnet),
        None,
    );
    MockProver::run(k, &circuit, vec![vec![]]).unwrap().assert_satisfied();
}
