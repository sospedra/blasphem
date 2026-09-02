package me.sospedra.blasphem

/** The JNI surface of `crates/blasphem-jni`. Every failure arrives as a RuntimeException carrying `CODE: detail`. */
internal object Native {
    init {
        System.loadLibrary("blasphem_jni")
    }

    external fun builderNew(detectLanguage: Boolean, grawlix: Boolean): Long
    external fun builderAdd(builder: Long, locale: String, pack: ByteArray, detect: ByteArray?)
    external fun builderBuild(builder: Long): Long
    external fun builderFree(builder: Long)
    external fun engineLocales(engine: Long): Array<String>
    external fun engineJudge(engine: Long, text: String): Judgement
    external fun engineFree(engine: Long)
}
