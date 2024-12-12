inspect_asm::bump_vec_u32::up::with_capacity:
	push r14
	push rbx
	push rax
	test rsi, rsi
	je .LBB0_1
	mov rax, rsi
	shr rax, 61
	jne .LBB0_4
	shl rsi, 2
	mov rcx, qword ptr [rdx]
	mov rax, qword ptr [rcx]
	mov rcx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	mov r8, rcx
	sub r8, rax
	cmp rsi, r8
	ja .LBB0_3
	test rax, rax
	je .LBB0_3
	and rcx, -4
.LBB0_0:
	sub rcx, rax
	shr rcx, 2
	jmp .LBB0_2
.LBB0_1:
	mov eax, 4
	xor ecx, ecx
.LBB0_2:
	mov qword ptr [rdi], rax
	mov qword ptr [rdi + 8], 0
	mov qword ptr [rdi + 16], rcx
	mov qword ptr [rdi + 24], rdx
	mov rax, rdi
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB0_3:
	mov rax, rsi
	mov esi, 4
	mov r14, rdi
	mov rdi, rdx
	mov rbx, rdx
	mov rdx, rax
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::prepare_allocation_range_in_another_chunk
	mov rdi, r14
	mov rcx, rdx
	mov rdx, rbx
	jmp .LBB0_0
.LBB0_4:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
