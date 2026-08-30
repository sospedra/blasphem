#include <jni.h>

#include "BlasphemReactNativeOnLoad.hpp"

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM* vm, void*) {
  return margelo::nitro::blasphem::initialize(vm);
}
