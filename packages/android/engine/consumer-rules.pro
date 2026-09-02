# The Rust side resolves these by name through JNI.
-keepclasseswithmembernames class me.sospedra.blasphem.Native { native <methods>; }
-keep class me.sospedra.blasphem.Judgement { <init>(boolean, double, java.lang.String, java.lang.String); }
