require "json"
require "pathname"
require "shellwords"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "BlasphemReactNative"
  s.version      = package["version"]
  s.summary      = "Local toxicity checks for React Native with sparse feature vectors and context rules."
  s.description  = package["description"]
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

  if defined?(Pod::Config)
    app_root = Pod::Config.instance.installation_root.parent.to_s
    if File.file?(File.join(app_root, "package.json"))
      script = File.join(__dir__, "scripts", "bundle-assets.cjs")
      assets = JSON.parse(Pod::Executable.execute_command("node", [script, "ios", app_root]))
      unless assets.fetch("files").empty?
        directory = Pathname.new(assets.fetch("directory")).relative_path_from(Pathname.new(__dir__)).to_s
        s.resource_bundles = { "BlasphemLocales" => assets.fetch("files").map { |name| File.join(directory, name) } }
        # A podspec phase runs in the pod target. Register a user phase on the
        # concrete consuming Podfile targets so only their final bundles sync.
        # During Podfile evaluation Config has not cached it yet. Reading the
        # accessor then would recursively evaluate the Podfile from this cwd.
        podfile = Pod::Config.instance.podfile if Pod::Config.instance.instance_variable_defined?(:@podfile)
        consumers = podfile ? podfile.target_definitions.values.select { |target|
          !target.root? && !target.abstract? && target.dependencies.any? { |dependency| dependency.root_name == s.name }
        } : []
        consumers.each do |consumer|
          phase_name = "Synchronize Blasphem locale resources"
          unless consumer.script_phases.any? { |phase| phase[:name] == phase_name }
            consumer.store_script_phase({
              :name => phase_name,
              :execution_position => :any,
              :always_out_of_date => "1",
              :script => <<~SH
                set -eu
                source=#{Shellwords.escape(assets.fetch("directory"))}
                : "${TARGET_BUILD_DIR:?}" "${UNLOCALIZED_RESOURCES_FOLDER_PATH:?}"
                destination="${TARGET_BUILD_DIR}/${UNLOCALIZED_RESOURCES_FOLDER_PATH}/BlasphemLocales.bundle"
                test -f "$source/bundle.json"
                test ! -L "$destination"
                mkdir -p "$destination"
                /usr/bin/rsync -a --delete --exclude Info.plist "$source/" "$destination/"
              SH
            })
          end
        end
      end
    end
  end

  s.pod_target_xcconfig = {
    "HEADER_SEARCH_PATHS" => "\"$(PODS_TARGET_SRCROOT)/cpp\"",
  }

  load "nitrogen/generated/ios/BlasphemReactNative+autolinking.rb"
  add_nitrogen_files(s)

  install_modules_dependencies(s)
end
