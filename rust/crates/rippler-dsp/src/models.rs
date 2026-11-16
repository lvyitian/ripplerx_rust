//! The acoustic models: frequency ratio tables and gain tables.
//!
//! Port of `src/dsp/Models.{h,cpp}`.
//!
//! The twelve models are described by a table of 64 frequency ratios each,
//! plus a table of 64 gains for the models that were measured rather than
//! derived.

/// The number of partials in every modal model.
pub const MAX_PARTIALS: usize = 64;

/// The frequency ratios of one modal model, one entry per partial.
pub type Ratios = [f64; MAX_PARTIALS];

/// The acoustic models a resonator can be tuned to.
///
/// The discriminants match the `a_model` and `b_model` parameters' choice
/// order, so a variant's discriminant is the value the plugin stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModalModel {
    /// A stretched string. The partials sit at whole multiples of the base.
    String = 0,
    /// A free-free beam.
    Beam = 1,
    /// A string squared, for inharmonic strings.
    Squared = 2,
    /// A circular membrane.
    Membrane = 3,
    /// A rectangular plate.
    Plate = 4,
    /// A circular drumhead.
    Drumhead = 5,
    /// A marimba bar, measured from a Chromaphone sample.
    Marimba = 6,
    /// An open tube. See the note on the table order below.
    OpenTube = 7,
    /// A closed tube. See the note on the table order below.
    ClosedTube = 8,
    /// A marimba bar, from the Faust modal model library.
    Marimba2 = 9,
    /// A Russian bell, from the Faust modal model library.
    Bell = 10,
    /// A djembe, modelled as a series of partials 200 Hz apart.
    Djembe = 11,
}

/// The free-free beam's roots, `bFree` in the C++.
///
/// `bFree[n]` is the nth solution of `cos(pi*x) = 1/cos(pi*x)`.
const BFREE: [f64; MAX_PARTIALS] = [
    1.50561873, 2.49975267, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5,
    16.5, 17.5, 18.5, 19.5, 20.5, 21.5, 22.5, 23.5, 24.5, 25.5, 26.5, 27.5, 28.5, 29.5, 30.5, 31.5,
    32.5, 33.5, 34.5, 35.5, 36.5, 37.5, 38.5, 39.5, 40.5, 41.5, 42.5, 43.5, 44.5, 45.5, 46.5, 47.5,
    48.5, 49.5, 50.5, 51.5, 52.5, 53.5, 54.5, 55.5, 56.5, 57.5, 58.5, 59.5, 60.5, 61.5, 62.5, 63.5,
    64.5,
];

/// The frequency ratios of the twelve models, `aModels` in the C++.
///
/// The tube tables are swapped relative to the enum. The C++ fills index 7
/// with the **closed** tube's harmonics (`1.0, 2.0, 3.0, ... 64.0`) and index
/// 8 with the **open** tube's harmonics (`1, 3, 5, ... 127`), while
/// `ModalModels` lists `OpenTube` at index 7 and `ClosedTube` at index 8. The
/// swap is reproduced here deliberately, so that the port is bit-identical to
/// the C++.
///
/// The effect is limited to the serial coupling frequency shift. For the tube
/// models the resonator only reads `model[0]`, which is `1.0` in both tables,
/// so the waveguide itself is unaffected. The difference shows up in
/// `Voice::calcFrequencyShifts`, which walks all 64 entries when the
/// resonators are coupled in series, so serial coupling on a tube model shifts
/// by a different amount than the model name suggests. Changing this is a
/// deliberate, separate change from the port.
const MODEL_RATIOS: [Ratios; 12] = [
    [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0,
        32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0,
        47.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 60.0, 61.0,
        62.0, 63.0, 64.0,
    ],
    [
        1.0,
        2.742118730723096,
        5.372437705395023,
        8.879796231350905,
        13.26432297929055,
        18.525894832548836,
        24.664464701923304,
        31.680011427264994,
        39.57480167993186,
        48.34386187557112,
        57.98998201587341,
        68.51312839987956,
        79.91327932335997,
        92.19042029669127,
        105.34454134738507,
        119.37563542989709,
        134.286605658006,
        150.07132597088105,
        166.73305349293955,
        184.27177735520195,
        202.68748915580662,
        221.9801823173097,
        242.14985163115497,
        263.1964929295172,
        285.1237904740257,
        307.92409320904443,
        331.6013888047332,
        356.1556713642617,
        381.58693597293706,
        407.89517850825996,
        435.08039549149447,
        463.1425839706985,
        492.0862467609114,
        521.9021136490643,
        552.5949669402571,
        584.164802709515,
        616.6116175480827,
        649.9354084840451,
        684.1361729168341,
        719.2139085629082,
        755.1739518807949,
        792.0053758900387,
        829.7137826702212,
        868.2991693165571,
        907.7615332399005,
        948.1008721264657,
        989.3171839034187,
        1031.4104667093834,
        1074.3868982163283,
        1118.233875919467,
        1162.9578340759736,
        1208.5587703942417,
        1255.0366827948008,
        1302.3915693871795,
        1350.6234284496627,
        1399.7322584115318,
        1449.7250824905104,
        1500.5876119645127,
        1552.3271202881517,
        1604.9436055754263,
        1658.4370660924033,
        1712.8075002427393,
        1768.0549065547818,
        1824.179283670064,
    ],
    [
        1.0,
        4.0,
        9.0,
        16.0,
        24.999999999999996,
        36.0,
        49.0,
        64.0,
        81.0,
        99.99999999999999,
        121.0,
        144.0,
        169.0,
        196.0,
        225.0,
        256.0,
        289.0,
        324.0,
        361.0,
        399.99999999999994,
        441.0,
        484.0,
        529.0,
        576.0,
        625.0,
        676.0,
        729.0,
        784.0,
        840.9999999999999,
        900.0,
        961.0,
        1024.0,
        1089.0,
        1156.0,
        1225.0,
        1296.0,
        1369.0,
        1444.0,
        1521.0,
        1599.9999999999998,
        1680.9999999999998,
        1764.0,
        1849.0,
        1936.0,
        2024.9999999999998,
        2116.0,
        2209.0,
        2304.0,
        2401.0,
        2500.0,
        2601.0,
        2704.0,
        2808.9999999999995,
        2916.0,
        3025.0,
        3136.0,
        3248.9999999999995,
        3363.9999999999995,
        3481.0,
        3600.0,
        3720.9999999999995,
        3844.0,
        3968.9999999999995,
        4096.0,
    ],
    [
        1.0,
        1.46109285817633,
        2.0065176070089703,
        2.583401188563129,
        3.17463993575823,
        3.773492277607296,
        4.376834180480457,
        4.983035133780558,
        1.6926924291749292,
        2.0,
        2.427204269735683,
        2.92218571635266,
        3.455943631122641,
        4.013035214017941,
        4.584962933678762,
        5.166802377126258,
        2.4441536557186816,
        2.6662106880321503,
        3.0,
        3.4128944891151236,
        3.8797198113361246,
        4.38327857452899,
        4.912287098306147,
        5.459352181097535,
        3.213415363586011,
        3.3853848583498585,
        3.6540595515354726,
        4.0,
        4.405039956758808,
        4.854408539471366,
        5.336920061454232,
        5.84437143270532,
        3.990195644111825,
        4.129945958302949,
        4.352904109387891,
        4.647109099145228,
        5.0,
        5.400083818557902,
        5.837665519855606,
        6.304942539213217,
        4.770823412246371,
        4.888307311437363,
        5.078077287524788,
        5.332421376064301,
        5.6426141771832246,
        6.0,
        6.3966736257417525,
        6.825788978230247,
        5.553676490089963,
        5.654919530531964,
        5.819745309187935,
        6.042953272833355,
        6.318359065318225,
        6.639470349789949,
        7.0,
        7.394184316141261,
        6.337930329019764,
        6.426830727172022,
        6.5723263585085,
        6.770769716699717,
        7.017670523558341,
        7.308119103070945,
        7.63714857122281,
        8.0,
    ],
    [
        1.0,
        2.134792340213877,
        4.026112907237005,
        6.673961701069386,
        10.078338721711017,
        14.239243969161897,
        19.156677443422033,
        24.83063914449142,
        2.8652076597861225,
        4.0,
        5.891320567023128,
        8.539169360855508,
        11.943546381497141,
        16.10445162894802,
        21.021885103208156,
        26.695846804277544,
        5.973887092762994,
        7.108679432976872,
        9.0,
        11.647848793832381,
        15.052225814474012,
        19.21313106192489,
        24.130564536185027,
        29.804526237254414,
        10.326038298930614,
        11.46083063914449,
        13.352151206167619,
        16.0,
        19.404377020641633,
        23.565282268092513,
        28.482715742352646,
        34.15667744342203,
        15.921661278288981,
        17.056453618502857,
        18.947774185525986,
        21.595622979358367,
        25.0,
        29.16090524745088,
        34.07833872171101,
        39.7523004227804,
        22.760756030838103,
        23.895548371051976,
        25.786868938075106,
        28.434717731907487,
        31.839094752549116,
        36.0,
        40.91743347426013,
        46.591395175329524,
        30.843322556577967,
        31.97811489679184,
        33.86943546381497,
        36.517284257647354,
        39.92166127828899,
        44.082566525739864,
        49.0,
        54.67396170106939,
        40.16936085550858,
        41.304153195722456,
        43.195473762745586,
        45.84332255657796,
        49.2476995772196,
        53.408604824670476,
        58.32603829893061,
        64.0,
    ],
    [
        1.0,
        1.593340505695112,
        2.1355487866494034,
        2.295417267427694,
        2.6530664045492145,
        2.9172954551172228,
        3.155464815408362,
        3.5001474903090264,
        3.5984846739581138,
        3.6474511791052775,
        4.058931883331434,
        4.131738159726707,
        4.230439127905234,
        4.6010445344331075,
        4.610051645437306,
        4.831885262930598,
        4.903280573212368,
        5.1307689067016575,
        5.412118429982582,
        5.5403985098530635,
        5.650842376925684,
        5.976540221648715,
        6.152609171589257,
        6.1631367313038865,
        6.208732130572546,
        6.528612451522295,
        6.746213299505839,
        6.848991602808508,
        7.0707081490386905,
        7.325257332462771,
        7.468242109085181,
        7.514500962483965,
        7.604536126938166,
        7.892520026843893,
        8.071028338967128,
        8.1568737689496,
        8.45000551018646,
        8.66047555520746,
        8.781093075730398,
        8.820447105611922,
        8.999214496283312,
        9.238840557670077,
        9.390589484063241,
        9.464339027734203,
        9.807815107462856,
        9.98784275554081,
        10.092254814868133,
        10.126502295693772,
        10.368705458854519,
        10.574713443493692,
        10.706875023386747,
        10.77153891878896,
        11.152639282954734,
        11.310212368186301,
        11.402312929615599,
        11.722758172320448,
        11.903823217314876,
        12.020976194473256,
        12.48894011894477,
        12.6291936518746,
        13.066558649839825,
        13.228284530761863,
        13.819314942198952,
        14.40316086180383,
    ],
    [
        1.0,
        3.9393939393939394,
        10.575757575757576,
        19.0,
        26.757575757575758,
        36.93939393939394,
        49.18181818181818,
        63.18181818181818,
        78.9090909090909,
        96.39393939393939,
        115.63636363636364,
        136.6060606060606,
        159.36363636363637,
        183.8181818181818,
        210.06060606060606,
        238.03030303030303,
        267.75757575757575,
        299.24242424242425,
        442.6363636363636,
        568.5454545454545,
        614.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
        100000.0,
    ],
    [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0,
        32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0,
        47.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 60.0, 61.0,
        62.0, 63.0, 64.0,
    ],
    [
        1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0, 31.0,
        33.0, 35.0, 37.0, 39.0, 41.0, 43.0, 45.0, 47.0, 49.0, 51.0, 53.0, 55.0, 57.0, 59.0, 61.0,
        63.0, 65.0, 67.0, 69.0, 71.0, 73.0, 75.0, 77.0, 79.0, 81.0, 83.0, 85.0, 87.0, 89.0, 91.0,
        93.0, 95.0, 97.0, 99.0, 101.0, 103.0, 105.0, 107.0, 109.0, 111.0, 113.0, 115.0, 117.0,
        119.0, 121.0, 123.0, 125.0, 127.0,
    ],
    [
        1.0,
        1.9203266581541947,
        2.3141717799876242,
        4.707864059430217,
        6.873884860855047,
        7.406986383228944,
        9.10045884559416,
        13.257060734683654,
        14.637949279677162,
        17.307596372965154,
        19.650542855838847,
        20.732751895404856,
        24.59892139973911,
        25.43424752188353,
        26.093486472917707,
        29.37789962712825,
        31.035072021669656,
        32.90951412040796,
        34.44950734864093,
        35.80195022645086,
        37.993540286176156,
        40.579442486151315,
        44.50494455748414,
        45.194115143390576,
        47.320534905911714,
        49.14126988675865,
        53.48072009994195,
        55.93681240825477,
        57.030803048781145,
        58.215013156121074,
        62.34982439046137,
        65.88462310021686,
        66.6255902741292,
        67.69718525877597,
        69.39628317024838,
        76.04970349637584,
        77.52887818992153,
        77.85993056285272,
        78.14597934292632,
        79.95811693928867,
        84.10067643372001,
        86.70823130573042,
        87.38710625836521,
        89.67687632609348,
        90.91553653516915,
        93.6067301599432,
        96.28667288650287,
        96.91767845145185,
        98.3304091612031,
        99.50071485659882,
        99.7700995704492,
        101.43321583574539,
        106.64471680109368,
        107.49702541110888,
        108.13599151724732,
        109.37252891533912,
        110.42332035234418,
        111.6513665065006,
        113.20420274118582,
        114.71776697266988,
        116.70896367551987,
        118.83177465936846,
        119.11410852022031,
        120.01948740483174,
    ],
    [
        1.0,
        1.0036779251227195,
        1.8972671990622025,
        1.9079712799472486,
        2.3187706059051942,
        2.3328522236061247,
        2.8855740347278185,
        2.886277382958458,
        3.3312110777346327,
        3.42510806652502,
        3.689603633965858,
        3.689918675360832,
        3.88643856692798,
        3.8907319217525096,
        3.964832588468019,
        4.001450655725694,
        4.237189537695069,
        4.248391823576819,
        4.835702249249029,
        5.189684225950619,
        5.252509341343687,
        5.254817202725474,
        5.684218624075024,
        5.6858744230346545,
        5.73928492929885,
        5.740083522602388,
        5.779368451901238,
        5.784035460473294,
        5.8637848926661285,
        5.910748040149461,
        6.4238259213129165,
        6.4503919701077,
        6.500820572935746,
        6.507597626199721,
        6.717085500769287,
        6.738303172393581,
        6.869997802036779,
        7.213759249761887,
        7.745622389918675,
        7.754780570005129,
        7.8062861748113415,
        7.9292988497325805,
        7.941460912887391,
        7.960656458348596,
        7.962268298043812,
        8.119276137445967,
        8.177668693677193,
        8.4181991354678,
        8.42948201333431,
        8.788482672723276,
        8.799252692504945,
        8.902996556524288,
        8.918528829950912,
        8.933841307055463,
        9.144406183603193,
        9.161183969521577,
        9.174078687083302,
        9.179573595135174,
        9.318631401567881,
        9.334163674994505,
        9.8958898087772,
        9.907685544728551,
        10.022639021173712,
        10.029672503480109,
    ],
    [
        1.0,
        3.857142857142857,
        6.714285714285714,
        9.571428571428571,
        12.428571428571429,
        15.285714285714286,
        18.142857142857142,
        21.0,
        23.857142857142858,
        26.714285714285715,
        29.571428571428573,
        32.42857142857143,
        35.285714285714285,
        38.142857142857146,
        41.0,
        43.857142857142854,
        46.714285714285715,
        49.57142857142857,
        52.42857142857143,
        55.285714285714285,
        58.142857142857146,
        61.0,
        63.857142857142854,
        66.71428571428571,
        69.57142857142857,
        72.42857142857143,
        75.28571428571429,
        78.14285714285714,
        81.0,
        83.85714285714286,
        86.71428571428571,
        89.57142857142857,
        92.42857142857143,
        95.28571428571429,
        98.14285714285714,
        101.0,
        103.85714285714286,
        106.71428571428571,
        109.57142857142857,
        112.42857142857143,
        115.28571428571429,
        118.14285714285714,
        121.0,
        123.85714285714286,
        126.71428571428571,
        129.57142857142858,
        132.42857142857142,
        135.28571428571428,
        138.14285714285714,
        141.0,
        143.85714285714286,
        146.71428571428572,
        149.57142857142858,
        152.42857142857142,
        155.28571428571428,
        158.14285714285714,
        161.0,
        163.85714285714286,
        166.71428571428572,
        169.57142857142858,
        172.42857142857142,
        175.28571428571428,
        178.14285714285714,
        181.0,
    ],
];

/// The gains of the measured models, `modelGains` in the C++.
///
/// The C++ declares four rows but initialises three, so the fourth row is all
/// zeros there too; that is reproduced here.
const MODEL_GAINS: [Ratios; 4] = [
    [
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ],
    [
        0.96623, 0.859578, 1.0, 0.788921, 0.722356, 0.828411, 0.804952, 0.321144, 0.716796,
        0.634043, 0.356832, 0.758356, 0.627535, 0.522109, 0.41653, 0.399465, 0.639828, 0.478843,
        0.395868, 0.589925, 0.789059, 0.525307, 0.814867, 0.641971, 0.554683, 0.417749, 0.288577,
        0.80181, 0.706824, 0.268298, 0.461017, 0.598288, 0.769713, 0.388368, 0.510933, 0.561361,
        0.495144, 0.562978, 0.155653, 0.814671, 0.540099, 0.636206, 0.263332, 0.324908, 0.0159624,
        0.409779, 0.849463, 0.390377, 0.807923, 0.195733, 0.368285, 0.133785, 0.315076, 0.561451,
        0.424994, 0.815392, 0.805216, 0.274915, 0.7907, 0.151032, 0.295099, 0.70326, 0.319596,
        0.591565,
    ],
    [
        0.925507, 0.59752, 0.0965671, 0.45412, 0.989773, 0.593498, 0.512541, 0.124241, 0.705411,
        0.292396, 0.673399, 0.302181, 0.026234, 0.286249, 0.556267, 1.0, 0.250426, 0.107711,
        0.427299, 0.336295, 0.616257, 0.21442, 0.0845294, 0.231363, 0.522724, 0.559114, 0.34847,
        0.854197, 0.835576, 0.735036, 0.288494, 0.117122, 0.409686, 0.363575, 0.484943, 0.170862,
        0.420531, 0.164793, 0.233847, 0.861232, 0.214037, 0.283462, 0.173153, 0.876122, 0.607809,
        0.294745, 0.143142, 0.332009, 0.491878, 0.626104, 0.384312, 0.527985, 0.201821, 0.297983,
        0.212535, 0.367379, 0.365935, 0.558277, 0.50738, 0.14265, 0.806237, 0.255278, 0.252357,
        0.117637,
    ],
    [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ],
];

/// The discriminant of a model, as an index into the tables.
const fn model_index(model: ModalModel) -> usize {
    model as usize
}

/// The frequency ratios of a model, as shipped.
pub fn ratios(model: ModalModel) -> &'static Ratios {
    &MODEL_RATIOS[model_index(model)]
}

/// The gains of a model, as shipped.
///
/// Only the two models that were measured have gains of their own; every
/// other model uses a flat gain table.
pub fn gains(model: ModalModel) -> &'static Ratios {
    match model {
        ModalModel::Marimba2 => &MODEL_GAINS[1],
        ModalModel::Bell => &MODEL_GAINS[2],
        _ => &MODEL_GAINS[0],
    }
}

/// Divides a table by its first entry, turning frequencies into ratios.
fn normalise(table: &mut Ratios) {
    let f0 = table[0];
    for value in table.iter_mut() {
        *value /= f0;
    }
}

/// Recomputes the beam model's ratios for a given ratio parameter.
pub fn beam(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for _n in 1..=8 {
            table[i] = ((m as f64).powf(4.0) + (ratio * BFREE[i]).powf(4.0)).sqrt();
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Recomputes the membrane model's ratios for a given ratio parameter.
pub fn membrane(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for n in 1..=8 {
            table[i] = ((m as f64).powf(2.0) + (ratio * n as f64).powf(2.0)).sqrt();
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Recomputes the plate model's ratios for a given ratio parameter.
pub fn plate(ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    let mut i = 0;
    for m in 1..=8 {
        for n in 1..=8 {
            table[i] = (m as f64).powf(2.0) + (ratio * n as f64).powf(2.0);
            i += 1;
        }
    }
    normalise(&mut table);
    table
}

/// Computes the djembe model's ratios for a given fundamental frequency.
pub fn djembe(freq: f64, ratio: f64) -> Ratios {
    let mut table = [0.0; MAX_PARTIALS];
    for (i, value) in table.iter_mut().enumerate() {
        *value = 1.0 + (200.0 * i as f64 * ratio) / freq;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_discriminants_match_the_choice_order() {
        assert_eq!(ModalModel::String as u8, 0);
        assert_eq!(ModalModel::Beam as u8, 1);
        assert_eq!(ModalModel::Squared as u8, 2);
        assert_eq!(ModalModel::Membrane as u8, 3);
        assert_eq!(ModalModel::Plate as u8, 4);
        assert_eq!(ModalModel::Drumhead as u8, 5);
        assert_eq!(ModalModel::Marimba as u8, 6);
        assert_eq!(ModalModel::OpenTube as u8, 7);
        assert_eq!(ModalModel::ClosedTube as u8, 8);
        assert_eq!(ModalModel::Marimba2 as u8, 9);
        assert_eq!(ModalModel::Bell as u8, 10);
        assert_eq!(ModalModel::Djembe as u8, 11);
    }

    #[test]
    fn every_model_has_a_full_table() {
        let models = [
            ModalModel::String,
            ModalModel::Beam,
            ModalModel::Squared,
            ModalModel::Membrane,
            ModalModel::Plate,
            ModalModel::Drumhead,
            ModalModel::Marimba,
            ModalModel::OpenTube,
            ModalModel::ClosedTube,
            ModalModel::Marimba2,
            ModalModel::Bell,
            ModalModel::Djembe,
        ];

        for model in models {
            let table = ratios(model);
            assert_eq!(table.len(), MAX_PARTIALS, "{model:?} has a short table");
            assert_eq!(table[0], 1.0, "{model:?} does not start at one");
        }
    }

    #[test]
    fn the_shipped_tables_match_the_cpp_values() {
        // The first and the last entry of every table, plus one entry from
        // the middle, all copied from `src/dsp/Models.cpp`.
        assert_eq!(ratios(ModalModel::String)[0], 1.0);
        assert_eq!(ratios(ModalModel::String)[63], 64.0);

        assert_eq!(ratios(ModalModel::Beam)[0], 1.0);
        assert_eq!(ratios(ModalModel::Beam)[1], 2.742118730723096);
        assert_eq!(ratios(ModalModel::Beam)[63], 1824.179283670064);

        assert_eq!(ratios(ModalModel::Squared)[0], 1.0);
        assert_eq!(ratios(ModalModel::Squared)[4], 24.999999999999996);
        assert_eq!(ratios(ModalModel::Squared)[63], 4096.0);

        assert_eq!(ratios(ModalModel::Membrane)[0], 1.0);
        assert_eq!(ratios(ModalModel::Membrane)[1], 1.46109285817633);
        assert_eq!(ratios(ModalModel::Membrane)[63], 8.0);

        assert_eq!(ratios(ModalModel::Plate)[0], 1.0);
        assert_eq!(ratios(ModalModel::Plate)[1], 2.134792340213877);
        assert_eq!(ratios(ModalModel::Plate)[63], 64.0);

        assert_eq!(ratios(ModalModel::Drumhead)[0], 1.0);
        assert_eq!(ratios(ModalModel::Drumhead)[1], 1.593340505695112);
        assert_eq!(ratios(ModalModel::Drumhead)[63], 14.40316086180383);

        assert_eq!(ratios(ModalModel::Marimba)[0], 1.0);
        assert_eq!(ratios(ModalModel::Marimba)[1], 3.9393939393939394);
        assert_eq!(ratios(ModalModel::Marimba)[63], 100000.0);

        // The tube tables are swapped relative to the enum: index 7 holds the
        // closed tube's harmonics and index 8 the open tube's.
        assert_eq!(ratios(ModalModel::OpenTube)[0], 1.0);
        assert_eq!(ratios(ModalModel::OpenTube)[1], 2.0);
        assert_eq!(ratios(ModalModel::OpenTube)[63], 64.0);

        assert_eq!(ratios(ModalModel::ClosedTube)[0], 1.0);
        assert_eq!(ratios(ModalModel::ClosedTube)[1], 3.0);
        assert_eq!(ratios(ModalModel::ClosedTube)[63], 127.0);

        assert_eq!(ratios(ModalModel::Marimba2)[0], 1.0);
        assert_eq!(ratios(ModalModel::Marimba2)[1], 1.9203266581541947);
        assert_eq!(ratios(ModalModel::Marimba2)[63], 120.01948740483174);

        assert_eq!(ratios(ModalModel::Bell)[0], 1.0);
        assert_eq!(ratios(ModalModel::Bell)[1], 1.0036779251227195);
        assert_eq!(ratios(ModalModel::Bell)[63], 10.029672503480109);

        assert_eq!(ratios(ModalModel::Djembe)[0], 1.0);
        assert_eq!(ratios(ModalModel::Djembe)[1], 3.857142857142857);
        assert_eq!(ratios(ModalModel::Djembe)[63], 181.0);

        // The gain rows.
        assert_eq!(gains(ModalModel::Marimba2)[0], 0.96623);
        assert_eq!(gains(ModalModel::Marimba2)[63], 0.591565);
        assert_eq!(gains(ModalModel::Bell)[0], 0.925507);
        assert_eq!(gains(ModalModel::Bell)[63], 0.117637);
    }

    #[test]
    fn the_beam_recalculation_uses_the_free_beam_table() {
        let recalculated = beam(0.5);

        // The table is normalised, so its first entry is always one.
        assert_eq!(recalculated[0], 1.0);

        // A ratio other than the shipped one gives a different table.
        assert_ne!(recalculated[..], ratios(ModalModel::Beam)[..]);

        // The free beam table drives the recomputation: the C++ uses
        // `bFree[i]`, indexed by the flat loop counter.
        let expected = ((1.0f64).powf(4.0) + (0.5 * BFREE[0]).powf(4.0)).sqrt();
        let f0 = ((1.0f64).powf(4.0) + (0.5 * BFREE[0]).powf(4.0)).sqrt();
        assert_eq!(recalculated[0], expected / f0);
    }

    #[test]
    fn the_membrane_and_plate_recalculations_differ_in_shape() {
        // The membrane takes the square root of the sum, the plate does not.
        assert_ne!(membrane(0.78), plate(0.78));

        // Both are normalised.
        assert_eq!(membrane(0.78)[0], 1.0);
        assert_eq!(plate(0.78)[0], 1.0);

        // The membrane uses the inner loop variable, the plate uses it too,
        // but only the membrane takes a square root, so the second entry of
        // the plate is the square of the membrane's.
        let squared = membrane(0.78)[1] * membrane(0.78)[1];
        assert!((plate(0.78)[1] - squared).abs() < 1e-12);
    }

    #[test]
    fn the_djembe_ratios_are_spaced_by_the_ratio() {
        let table = djembe(200.0, 1.0);

        assert_eq!(table[0], 1.0);

        let spacing = 200.0 * 1.0 / 200.0;
        for i in 1..MAX_PARTIALS {
            assert!(
                (table[i] - table[i - 1] - spacing).abs() < 1e-12,
                "entry {i} is out of step"
            );
        }
    }

    #[test]
    fn only_two_models_have_gains_of_their_own() {
        let flat = gains(ModalModel::String);
        assert!(flat.iter().all(|gain| *gain == 1.0));

        let marimba2 = gains(ModalModel::Marimba2);
        let bell = gains(ModalModel::Bell);

        assert_ne!(marimba2, flat);
        assert_ne!(bell, flat);
        assert_ne!(bell, marimba2);

        // Every model other than the two measured ones uses the flat table.
        for model in [
            ModalModel::Beam,
            ModalModel::Squared,
            ModalModel::Membrane,
            ModalModel::Plate,
            ModalModel::Drumhead,
            ModalModel::Marimba,
            ModalModel::OpenTube,
            ModalModel::ClosedTube,
            ModalModel::Djembe,
        ] {
            assert_eq!(gains(model), flat, "{model:?} has gains of its own");
        }
    }
}
