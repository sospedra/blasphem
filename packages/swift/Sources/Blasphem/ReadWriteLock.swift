import Foundation

final class ReadWriteLock: @unchecked Sendable {
    private let lock: UnsafeMutablePointer<pthread_rwlock_t>

    init() {
        lock = .allocate(capacity: 1)
        lock.initialize(to: pthread_rwlock_t())
        pthread_rwlock_init(lock, nil)
    }

    deinit {
        pthread_rwlock_destroy(lock)
        lock.deinitialize(count: 1)
        lock.deallocate()
    }

    func read<Result>(_ body: () throws -> Result) rethrows -> Result {
        pthread_rwlock_rdlock(lock)
        defer { pthread_rwlock_unlock(lock) }
        return try body()
    }

    func write<Result>(_ body: () throws -> Result) rethrows -> Result {
        pthread_rwlock_wrlock(lock)
        defer { pthread_rwlock_unlock(lock) }
        return try body()
    }
}
