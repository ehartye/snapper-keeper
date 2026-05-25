fn main() {
    #[cfg(windows)]
    {
        // embed-resource invokes RC.EXE on the .rc resource script.
        // The .rc declares RT_MANIFEST id 1 pointing at the side-by-side
        // manifest XML, which RC.EXE then embeds into the linked .exe.
        let _ = embed_resource::compile("smoke-target.rc", embed_resource::NONE);
    }
}
