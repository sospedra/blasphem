require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "BlasphemReactNative"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.homepage     = "https://github.com/sospedra/blasphem"
  s.license      = package["license"]
  s.authors      = "Rubén Sospedra"
  s.platforms    = { :ios => "15.1" }
  s.source       = { :git => "https://github.com/sospedra/blasphem.git", :tag => "#{s.version}" }

  s.source_files = [
    "ios/**/*.{swift,h,m,mm,cpp}",
    "cpp/**/*.{h,hpp,cpp}",
  ]
  s.private_header_files = ["cpp/**/*.{h,hpp}"]

  # The Rust engine behind the C ABI in cpp/blasphem.h. Built by `node scripts/rust.mjs`.
  s.vendored_frameworks = "ios/BlasphemFFI.xcframework"

  s.pod_target_xcconfig = {
    "HEADER_SEARCH_PATHS" => "\"$(PODS_TARGET_SRCROOT)/cpp\"",
  }

  load "nitrogen/generated/ios/BlasphemReactNative+autolinking.rb"
  add_nitrogen_files(s)

  install_modules_dependencies(s)
end
