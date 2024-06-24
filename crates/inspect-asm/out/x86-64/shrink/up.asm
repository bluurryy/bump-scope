inspect_asm::shrink::up:
	mov rax, rsi
	lea rdx, [r8 - 1]
	test rdx, rsi
	jne .LBB0_1
	lea rsi, [rax + rcx]
	mov rdx, qword ptr [rdi]
	cmp rsi, qword ptr [rdx]
	je .LBB0_0
	mov rdx, rcx
	ret
.LBB0_0:
	lea rcx, [rax + r9]
	mov qword ptr [rdx], rcx
	mov rdx, r9
	ret
.LBB0_1:
	push rax
	mov rsi, rax
	mov rdx, rcx
	mov rcx, r8
	mov r8, r9
	call bump_scope::allocator::shrink::shrink_unfit
	add rsp, 8
	ret
