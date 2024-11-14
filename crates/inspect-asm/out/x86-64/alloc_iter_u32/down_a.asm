inspect_asm::alloc_iter_u32::down_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov rbx, rdi
	test rdx, rdx
	je .LBB0_4
	mov rax, rdx
	shr rax, 61
	jne .LBB0_10
	mov r14, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rbx]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r15, rsi
	ja .LBB0_9
	sub rax, r15
	mov qword ptr [rcx], rax
.LBB0_0:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rbx
	xor r12d, r12d
	mov rbx, rsp
	xor edx, edx
	jmp .LBB0_2
.LBB0_1:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB0_3
.LBB0_2:
	mov ebp, dword ptr [r14 + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_1
	mov rdi, rbx
	call bump_scope::bump_vec::BumpVec<T,A>::generic_grow_amortized
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_1
.LBB0_3:
	mov rax, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rbx]
	cmp qword ptr [rcx], rax
	jne .LBB0_8
	jmp .LBB0_5
.LBB0_4:
	mov qword ptr [rsp + 16], rdx
	mov eax, 4
	xor edx, edx
	mov rcx, qword ptr [rbx]
	cmp qword ptr [rcx], rax
	jne .LBB0_8
.LBB0_5:
	mov r15, rdx
	lea rdx, [4*rdx]
	mov rcx, qword ptr [rsp + 16]
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
	jbe .LBB0_6
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_7
.LBB0_6:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_7:
	mov rax, qword ptr [rbx]
	mov qword ptr [rax], r14
	test r14, r14
	mov rax, r12
	cmovne rax, r14
	mov rdx, r15
.LBB0_8:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_9:
	mov rdi, rbx
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r12
	jmp .LBB0_0
.LBB0_10:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rdx, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rdx
	jne .LBB0_11
	mov rsi, qword ptr [rsp + 16]
	lea rdx, [rdx + 4*rsi]
	add rdx, 3
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB0_11:
	mov rdi, rax
	call _Unwind_Resume@PLT
