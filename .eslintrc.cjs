module.exports = {
    root: true,
    parser: '@typescript-eslint/parser',
    plugins: ['@typescript-eslint'],
    extends: [
        'eslint:recommended',
        'plugin:@typescript-eslint/recommended',
        'plugin:diff/diff',
        'prettier',
    ],
    parserOptions: {
        project: ['./tsconfig.json'],
    },
    rules: {
        'import/prefer-default-export': 'off',
        '@typescript-eslint/naming-convention': [
            'error',
            {
                selector: 'default',
                format: ['camelCase'],
                leadingUnderscore: 'allow',
                trailingUnderscore: 'forbid',
            },
            {
                selector: 'variable',
                format: ['camelCase', 'PascalCase', 'UPPER_CASE'],
                leadingUnderscore: 'allow',
                trailingUnderscore: 'forbid',
            },
            {
                selector: 'typeLike',
                format: ['PascalCase'],
            },
        ],
        'no-unused-vars': 'off', // prefer @typescript-eslint/no-unused-vars
        '@typescript-eslint/no-unused-vars': [
            'error',
            {
                argsIgnorePattern: '^_',
                varsIgnorePattern: '^_',
                caughtErrorsIgnorePattern: '^_',
            },
        ],
        '@typescript-eslint/no-namespace': 'off',
    },
    overrides: [
        {
            files: ['**/*.ts', '**/*.tsx'],
            rules: {
                '@typescript-eslint/naming-convention': [
                    'error',
                    {
                        selector: 'objectLiteralProperty',
                        format: null,
                        custom: { regex: '.+', match: true },
                    },
                ],
            },
        },
        {
            files: ['**/sst-env.d.ts', 'sst.config.ts', '**/infra/*.ts'],
            rules: {
                '@typescript-eslint/triple-slash-reference': 'off',
            },
        },
        {
            // Define the configuration for `.astro` file.
            files: ['*.astro'],
            // Allows Astro components to be parsed.
            parser: 'astro-eslint-parser',
            // Parse the script in `.astro` as TypeScript by adding the following configuration.
            // It's the setting you need when using TypeScript.
            parserOptions: {
                parser: '@typescript-eslint/parser',
                extraFileExtensions: ['.astro'],
            },
            rules: {
                // override/add rules settings here, such as:
                // "astro/no-set-html-directive": "error"
                '@typescript-eslint/naming-convention': 'off',
            },
        },
    ],
};
