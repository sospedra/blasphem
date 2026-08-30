cmake_minimum_required(VERSION 3.9.0)
project(BlasphemReactNative)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(
  BlasphemReactNative SHARED
  src/main/cpp/cpp-adapter.cpp
  ../cpp/HybridBlasphemEngine.cpp
  ../cpp/HybridBlasphemEngineBuilder.cpp
)

include_directories(../cpp)

# The Nitro glue: generated specs, fbjni, JSI, NitroModules.
include(${CMAKE_SOURCE_DIR}/../nitrogen/generated/android/BlasphemReactNative+autolinking.cmake)

# The Rust engine, one static archive per ABI, produced by `node scripts/rust.mjs`.
add_library(blasphem_ffi STATIC IMPORTED)
set_target_properties(
  blasphem_ffi PROPERTIES
  IMPORTED_LOCATION ${CMAKE_SOURCE_DIR}/libs/${ANDROID_ABI}/libblasphem_ffi.a
)
target_link_libraries(BlasphemReactNative blasphem_ffi)
