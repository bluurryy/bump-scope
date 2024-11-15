inspect_asm::alloc_iter_u32::try_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	test rdx, rdx
	je .LBB0_1
	mov rax, rdx
	shr rax, 61
	je .LBB0_2
.LBB0_0:
	xor eax, eax
	jmp .LBB0_10
.LBB0_1:
	mov qword ptr [rsp + 24], rdx
	mov eax, 4
	xor ebx, ebx
	mov rcx, qword ptr [rdi]
	cmp qword ptr [rcx], rax
	jne .LBB0_10
	jmp .LBB0_7
.LBB0_2:
	mov r14, rsi
	lea r12, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r12, rsi
	ja .LBB0_12
	sub rax, r12
	and rax, -4
	mov qword ptr [rcx], rax
	je .LBB0_12
.LBB0_3:
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rdi
	xor r13d, r13d
	lea r15, [rsp + 8]
	xor ebx, ebx
	jmp .LBB0_5
.LBB0_4:
	mov dword ptr [rax + 4*rbx], ebp
	inc rbx
	mov qword ptr [rsp + 16], rbx
	add r13, 4
	cmp r12, r13
	je .LBB0_6
.LBB0_5:
	mov ebp, dword ptr [r14 + r13]
	cmp qword ptr [rsp + 24], rbx
	jne .LBB0_4
	mov rdi, r15
	call bump_scope::bump_vec::BumpVec<T,A>::generic_grow_amortized
	test al, al
	jne .LBB0_11
	mov rax, qword ptr [rsp + 8]
	mov rbx, qword ptr [rsp + 16]
	jmp .LBB0_4
.LBB0_6:
	mov rax, qword ptr [rsp + 8]
	mov rdi, qword ptr [rsp + 32]
	mov rcx, qword ptr [rdi]
	cmp qword ptr [rcx], rax
	jne .LBB0_10
.LBB0_7:
	mov r15, rdi
	lea rdx, [4*rbx]
	mov rcx, qword ptr [rsp + 24]
	lea rcx, [rax + 4*rcx]
	xor r14d, r14d
	sub rcx, rdx
	cmovae r14, rcx
	and r14, -4
	lea rcx, [rdx + rax]
	mov rdi, r14
	mov r12, rax
	mov rsi, rax
	cmp rcx, r14
	jbe .LBB0_8
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_9
.LBB0_8:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_9:
	mov rax, qword ptr [r15]
	mov qword ptr [rax], r14
	test r14, r14
	mov rax, r12
	cmovne rax, r14
.LBB0_10:
	mov rdx, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_11:
	mov rax, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 32]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rax
	jne .LBB0_0
	mov rdx, qword ptr [rsp + 24]
	lea rax, [rax + 4*rdx]
	mov qword ptr [rcx], rax
	jmp .LBB0_0
.LBB0_12:
	mov rbx, rdi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r15
	mov rdi, rbx
	test rax, rax
	jne .LBB0_3
	jmp .LBB0_0
	mov rcx, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 32]
	mov rdx, qword ptr [rdx]
	cmp qword ptr [rdx], rcx
	jne .LBB0_13
	mov rsi, qword ptr [rsp + 24]
	lea rcx, [rcx + 4*rsi]
	mov qword ptr [rdx], rcx
.LBB0_13:
	mov rdi, rax
	call _Unwind_Resume@PLT
