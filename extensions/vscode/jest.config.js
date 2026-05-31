module.exports = {
    preset: 'ts-jest',
    testEnvironment: 'node',
    testMatch: ['**/src/**/*.test.ts'],
    moduleNameMapper: {
        '^vscode$': '<rootDir>/src/__mocks__/vscode.ts',
    },
    transform: {
        '^.+\\.ts$': ['ts-jest', { isolatedModules: true }],
    },
};
