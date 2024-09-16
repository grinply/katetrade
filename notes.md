// does the parsed file fit into main memory ? how much memory is required ?
8 bytes - price=f64
4 bytes - trade_number=i32
4 bytes - qty=i32
4 bytes - vol=f32
4 bytes - date=i32
4 bytes - time=i32
1 byte - buy_agent=u8
1 byte - sell_agent=u8
1 byte - trade_type=u8
1 byte - is edit trade=boolean


// does the files fit in the available disk space ?
let disks = Disks::new_with_refreshed_list();
for disk in disks.list() {
    println!(
        "[{:?}] {}GB - {:?}",
        disk.name(),
        (disk.available_space() / 1_000_000_000),
        disk.mount_point()
    );
}
