/// Used to build `SfxInfo`
pub(crate) struct SfxInfoBase {
    pub name: &'static str,
    pub priority: i32,
}

impl SfxInfoBase {
    pub(crate) const fn new(name: &'static str, priority: i32) -> Self {
        Self { name, priority }
    }
}

/// The ordering here should match the `SfxName` ordering
pub(crate) const SFX_INFO_BASE: [SfxInfoBase; 109] = [
    SfxInfoBase::new("none", 0),
    SfxInfoBase::new("pistol", 64),
    SfxInfoBase::new("shotgn", 64),
    SfxInfoBase::new("sgcock", 64),
    SfxInfoBase::new("dshtgn", 64),
    SfxInfoBase::new("dbopn", 64),
    SfxInfoBase::new("dbcls", 64),
    SfxInfoBase::new("dbload", 64),
    SfxInfoBase::new("plasma", 64),
    SfxInfoBase::new("bfg", 64),
    SfxInfoBase::new("sawup", 64),
    SfxInfoBase::new("sawidl", 118),
    SfxInfoBase::new("sawful", 64),
    SfxInfoBase::new("sawhit", 64),
    SfxInfoBase::new("rlaunc", 64),
    SfxInfoBase::new("rxplod", 70),
    SfxInfoBase::new("firsht", 70),
    SfxInfoBase::new("firxpl", 70),
    SfxInfoBase::new("pstart", 100),
    SfxInfoBase::new("pstop", 100),
    SfxInfoBase::new("doropn", 100),
    SfxInfoBase::new("dorcls", 100),
    SfxInfoBase::new("stnmov", 119),
    SfxInfoBase::new("swtchn", 78),
    SfxInfoBase::new("swtchx", 78),
    SfxInfoBase::new("plpain", 96),
    SfxInfoBase::new("dmpain", 96),
    SfxInfoBase::new("popain", 96),
    SfxInfoBase::new("vipain", 96),
    SfxInfoBase::new("mnpain", 96),
    SfxInfoBase::new("pepain", 96),
    SfxInfoBase::new("slop", 78),
    SfxInfoBase::new("itemup", 78),
    SfxInfoBase::new("wpnup", 78),
    SfxInfoBase::new("oof", 96),
    SfxInfoBase::new("telept", 32),
    SfxInfoBase::new("posit1", 98),
    SfxInfoBase::new("posit2", 98),
    SfxInfoBase::new("posit3", 98),
    SfxInfoBase::new("bgsit1", 98),
    SfxInfoBase::new("bgsit2", 98),
    SfxInfoBase::new("sgtsit", 98),
    SfxInfoBase::new("cacsit", 98),
    SfxInfoBase::new("brssit", 94),
    SfxInfoBase::new("cybsit", 92),
    SfxInfoBase::new("spisit", 90),
    SfxInfoBase::new("bspsit", 90),
    SfxInfoBase::new("kntsit", 90),
    SfxInfoBase::new("vilsit", 90),
    SfxInfoBase::new("mansit", 90),
    SfxInfoBase::new("pesit", 90),
    SfxInfoBase::new("sklatk", 70),
    SfxInfoBase::new("sgtatk", 70),
    SfxInfoBase::new("skepch", 70),
    SfxInfoBase::new("vilatk", 70),
    SfxInfoBase::new("claw", 70),
    SfxInfoBase::new("skeswg", 70),
    SfxInfoBase::new("pldeth", 32),
    SfxInfoBase::new("pdiehi", 32),
    SfxInfoBase::new("podth1", 70),
    SfxInfoBase::new("podth2", 70),
    SfxInfoBase::new("podth3", 70),
    SfxInfoBase::new("bgdth1", 70),
    SfxInfoBase::new("bgdth2", 70),
    SfxInfoBase::new("sgtdth", 70),
    SfxInfoBase::new("cacdth", 70),
    SfxInfoBase::new("skldth", 70),
    SfxInfoBase::new("brsdth", 32),
    SfxInfoBase::new("cybdth", 32),
    SfxInfoBase::new("spidth", 32),
    SfxInfoBase::new("bspdth", 32),
    SfxInfoBase::new("vildth", 32),
    SfxInfoBase::new("kntdth", 32),
    SfxInfoBase::new("pedth", 32),
    SfxInfoBase::new("skedth", 32),
    SfxInfoBase::new("posact", 120),
    SfxInfoBase::new("bgact", 120),
    SfxInfoBase::new("dmact", 120),
    SfxInfoBase::new("bspact", 100),
    SfxInfoBase::new("bspwlk", 100),
    SfxInfoBase::new("vilact", 100),
    SfxInfoBase::new("noway", 78),
    SfxInfoBase::new("barexp", 60),
    SfxInfoBase::new("punch", 64),
    SfxInfoBase::new("hoof", 70),
    SfxInfoBase::new("metal", 70),
    SfxInfoBase::new("chgun", 64),
    SfxInfoBase::new("tink", 60),
    SfxInfoBase::new("bdopn", 100),
    SfxInfoBase::new("bdcls", 100),
    SfxInfoBase::new("itmbk", 100),
    SfxInfoBase::new("flame", 32),
    SfxInfoBase::new("flamst", 32),
    SfxInfoBase::new("getpow", 60),
    SfxInfoBase::new("bospit", 70),
    SfxInfoBase::new("boscub", 70),
    SfxInfoBase::new("bossit", 70),
    SfxInfoBase::new("bospn", 70),
    SfxInfoBase::new("bosdth", 70),
    SfxInfoBase::new("manatk", 70),
    SfxInfoBase::new("mandth", 70),
    SfxInfoBase::new("sssit", 70),
    SfxInfoBase::new("ssdth", 70),
    SfxInfoBase::new("keenpn", 70),
    SfxInfoBase::new("keendt", 70),
    SfxInfoBase::new("skeact", 70),
    SfxInfoBase::new("skesit", 70),
    SfxInfoBase::new("skeatk", 70),
    SfxInfoBase::new("radio", 60),
];
