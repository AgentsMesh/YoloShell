use yolo_release_contracts::*;

pub(crate) fn capture_script(startup: &str, links: &str) -> String {
    capture_target_script(startup, links, LINUX_BUILD_TARGET)
}

pub(crate) fn capture_target_script(startup: &str, links: &str, target: &str) -> String {
    format!(
        "runtime_output=$({DOCKER_BAZEL} {startup} {BAZEL_CQUERY_VERB} \
         {BAZEL_LOCKFILE_ARGUMENT} {links} {BAZEL_OPTIMIZED_ARGUMENT} {SERIAL_JOBS_ARGUMENT} \
         {BAZEL_QUERY_STARLARK_OUTPUT_ARGUMENT} '{BAZEL_QUERY_EXECUTABLE_EXPRESSION_ARGUMENT}' \
         {target})\n\
         case \"$runtime_output\" in {BAZEL_EXECROOT_OUTPUT_PREFIX}*) ;; *) exit 1 ;; esac\n\
         case \"/$runtime_output/\" in */../*|*/./*|*//*|*$'\\n'*|*$'\\r'*) exit 1 ;; esac\n\
         runtime_executable={DOCKER_SYMLINK_PREFIX}{BAZEL_OUTPUT_LINK_SUFFIX}\
         ${{runtime_output#{BAZEL_EXECROOT_OUTPUT_PREFIX}}}\n\
         test -f \"$runtime_executable\"\n\
         test -x \"$runtime_executable\"\n\
         test ! -L \"$runtime_executable\""
    )
}
