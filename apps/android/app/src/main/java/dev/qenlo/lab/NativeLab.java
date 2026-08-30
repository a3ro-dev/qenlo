package dev.qenlo.lab;

final class NativeLab {
    static { System.loadLibrary("qenlo_mobile"); }
    static native String run(String profile);
    private NativeLab() {}
}
